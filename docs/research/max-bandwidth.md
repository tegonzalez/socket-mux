---
id: note:repo-evals/2026-10-21-socket-mux-max-bandwidth
title: socket-mux max-bandwidth design (Win64 send, Linux receive, Rust)
created: 2026-10-21T17:05:00-07:00
updated: 2026-10-21T17:05:00-07:00
tags: [repo-eval, performance, dev-tools]
status: active
related: project:socket-mux, note:repo-evals/2026-10-21-socket-mux-rust-crates (not copied; remains in Scout's research graph), [prior art](prior-art.md)
sources: [sources/](sources/) ([excerpts.md](sources/excerpts.md), [index.json](sources/index.json))
---

# socket-mux: structuring for maximum bandwidth (2026-10-21)

**Asked via:** the MUX DMUX bot, 16:49 PT.
**Design:** a per-datagram sequence header, no retransmission, and lane k on UDP port base+k (N>=3). Windows takes SCPI over localhost TCP and sends UDP out a 10G+ NIC. Linux receives the lanes and reassembles.
**Evidence so far:** single-stream iperf3 reaches about 3 Gbps, and multi-thread iperf3 gets near line rate.
**Tags used below:**
- **[proven]**: independent measurement, or documentation of the behavior.
- **[vendor-claimed]**: stated by the vendor or kernel author.
- **[speculative]**: Scout's reasoning.

**Bottom line:** per-packet syscall and CPU cost is the limit, not the wire. Batch sends into 64 KB super-buffers (Windows USO, Linux GRO), give each lane one core pinned on its RSS queue, and keep zero-copy for later. [speculative synthesis of the sources below]

Key arithmetic: 10 Gbit/s ≈ 1.25 GB/s. That is about 830k datagrams/s at 1472 B payload, or about 140k/s with 9000 B jumbo frames. [arithmetic]

## Numbers found
| Claim | Value | Tag | Source |
|---|---|---|---|
| IOCP vs RIO UDP receive, 10GbE, Server 2012 RC, 8 threads | 384k vs 492k datagrams/s (hybrid RIO 501k) | proven (single blogger, 2012) | serverframework.com/asynchronousevents/2012/08/winsock-registered-io-io-completion-port-performance.html |
| Same author, rough | IOCP 350k; RIO 450k naive, 920–970k optimized | proven (rough, unchecked by the author) | serverframework.com/.../2012/07/windows-8-registered-io-performance---finally-using-a-full-10-gigabit-link.html |
| RIO on Win10 (user report) | 180k pps per socket, 260k pps across RSS queues; the 2012 author reproduced the disappointment | proven (anecdotal) | learn.microsoft.com/en-us/answers/questions/268716 |
| WFP callouts on the send path | "easily degrading egress rate by 40%" | vendor-claimed (answerer in a Microsoft Q&A thread) | same thread |
| RIO, one steered flow, 64 B frames | ~1 Mpps; 2.3 Mpps with UDP_SEND_MSG_SIZE on 64 KB buffers | vendor-claimed (same answer) | same thread |
| Linux UDP GSO (patch cover letter) | connected UDP tx 825 MB/s | proven (kernel dev measurement) | lists.openwall.net/netdev/2018/04/18/189 |
| wireguard-go with UDP GSO/GRO | 13.0 Gb/s on i5-12400 bare metal | vendor-claimed (Tailscale) | tailscale.com/blog/more-throughput |
| io_uring zcrx | 200G saturated with 1 core; 188 Gbit on 1 core (netdev 0x19) | vendor-claimed (Jens Axboe / Jamal) | github.com/torvalds/linux/commit/ca0b04ba0b35 |
| AF_XDP i40e, 64 B rxdrop | 8.2 Mpps copy vs 15.0 Mpps zero-copy; at 1500 B both 3.3 Mpps | proven (patch-set measurement, 40G) | lists.openwall.net/netdev/2018/08/28/62 |
| QUIC on 10G links | 90 Mbit/s – 4.9 Gbit/s by implementation; default buffers "too small" by ≥10x | proven (TUM paper) | arxiv.org/pdf/2309.16395 |

**No independent numbers were found for:**
- Windows USO throughput
- Windows URO
- loopback TCP intake throughput on current Windows
- UDP-only io_uring multishot vs recvmmsg

## 1. Win64 send, ranked by gain per effort
1. **Thread per lane**, pinned to the core that services its RSS queue. Use blocking or overlapped send with no async runtime on the hot path. [speculative; matches the multi-thread iperf3 evidence]
2. **USO.** Call `WSASetUdpSendMessageSize` (UDP_SEND_MSG_SIZE) and send 64 KB buffers of MSS-sized segments.
   - Requirements: Windows 10 2004 / Server 2004 (build 19041) or later [proven, MS docs]. NIC and driver support, checked with `Get-NetAdapterUso` and turned on with `Enable-NetAdapterUso` [proven, MS docs]. **NIC/driver dependent.**
   - Not applied to loopback [proven].
   - Fit with the sequence header: each segment is its own datagram. Either each segment carries its own sequence header inside its MSS bytes, or the drivers lack SubMssFinalSegmentSupported, in which case the buffer length must be a multiple of the MSS [proven, MS docs]. Lay the buffer out as [hdr|payload]×k.
3. **Jumbo frames (MTU 9000) on both ends.** This cuts pps about 6x. **Needs a switch/NIC path that carries jumbo.** [speculative gain; arithmetic]
4. **SO_SNDBUF** large (several MB), and keep send queue depth bounded. [speculative]
5. **WFP filters and the firewall on the send path.** Measure with them off, because one report says 40% [vendor-claimed]. They may be policy-locked on an instrument PC.
6. **RIO** (registered buffers and completion queues, via windows-sys `RIO_EXTENSION_FUNCTION_TABLE`). Rank it last.
   - The 2012 gains were about 1.3x to 2.5x [proven, old]. A newer report found no gain [proven, anecdotal].
   - msquic **removed** its experimental RIO datapath in Aug 2025: "Performance tests showed limited upside … where RIO should be better than IOCP are scenarios where XDP, DPDK, RDMA, etc. will be even better" [vendor-claimed, github.com/microsoft/msquic/pull/5328].
   - The alternative is XDP for Windows (microsoft/xdp-for-windows), whose msquic send side is fast. Its receive is "unexpectedly slow" (open issue msquic#5980, 2026-05) [vendor-claimed]. **NIC/driver dependent.**
7. **RSS, interrupt moderation and NUMA.** On send, the main point is to keep each lane thread, its buffers and the NIC on one NUMA node, the one local to the NIC's PCIe slot. [speculative] Interrupt moderation matters mostly for RX and TX completions.
8. **Loopback TCP intake.**
   - `SIO_LOOPBACK_FAST_PATH` is **deprecated**. Microsoft says normal loopback "has been improved quite a bit", but notes "if you do measure a valuable perf benefit … you may certainly use it". It is skipped when WFP callouts are present [proven, MS docs + MS answer].
   - Use a large SO_RCVBUF, then one reader thread that reads straight into the slab (below). Never copy again after that. [speculative]

## 2. Linux receive, ranked
1. **One socket per lane port**, each on its own thread. Pin with `ethtool -N` n-tuple rules (dst-port base+k → queue k) plus IRQ affinity for queue k to core k, and the receiver thread on core k.
   - Needs n-tuple / ntuple-filter support: **NIC dependent**, but common on 10G+.
   - SO_REUSEPORT and BPF steering are not needed, because each lane already has its own port. [speculative]
   - Without n-tuple, RSS hashes the 4-tuple, so the lanes may collide on one queue. Check with `ethtool -S`. [speculative]
2. **UDP_GRO sockopt + recvmmsg.** GRO coalesces same-flow, same-size datagrams into one 64 KB read. Split it with the `UDP_GRO` cmsg gso_size. Keep all segments equal-size so GRO works. [proven behavior per the TUM paper and Tailscale; gain vendor-claimed]
3. **rmem_max / SO_RCVBUF** of tens of MB, and `ethtool -G rx` at the maximum. This is the top cause of drops (UdpRcvbufErrors). [proven: the TUM paper says defaults are too small]
4. **Busy polling:** `napi_defer_hard_irqs` + `gro_flush_timeout`, or SO_BUSY_POLL. It trades CPU for fewer IRQs [proven, kernel docs]. Gain for this workload unmeasured [speculative].
5. **io_uring multishot recvmsg** with a provided buffer ring (`io_uring_prep_recvmsg_multishot`, or recv + `IORING_RECVSEND_BUNDLE` when no cmsg is needed) [proven, man page]. No UDP number vs recvmmsg+GRO was found.
6. **Defer zero-copy.**
   - io_uring zcrx (Linux 6.15+) is TCP-oriented (header/data split, "flow-type tcp6"). The supported NICs are Broadcom Thor BCM95750x (FW≥232) and Mellanox ConnectX-7 (FW≥28.42), plus idpf per Jamal [proven docs, vendor-claimed perf]. UDP support was not found.
   - AF_XDP zero-copy needs driver support (i40e yes; ice and mlx5 depend on configuration). It bypasses the UDP stack, so you parse headers yourself [proven].
   - **Both are NIC dependent.**

## 3. Rust structure
- Hot path: plain `std::thread` per lane plus core pinning (`core_affinity`, or SetThreadAffinityMask / sched_setaffinity via windows-sys and libc). Use tokio only for the control plane, or not at all. Thread-per-lane beats one current_thread runtime here, because one core is what caps single-stream iperf3. [speculative] This revises the morning pick (tokio current_thread) for the data path.
- Windows: `windows-sys` covers WSASetUdpSendMessageSize, the RIO function table and SIO_UDP_CONNRESET [proven, docs.rs]. `socket2` covers buffers. `quinn-udp` does USO (UDP_SEND_MSG_SIZE) and URO (UDP_RECV_MAX_COALESCED_SIZE). URO is **off by default on Windows** after failures (missing metadata with WSL installed). URO needs Windows 11 24H2 + NIC [proven, PR quinn-rs/quinn#2092, MS docs]. No maintained RIO crate was found.
- Linux: `quinn-udp` (GSO via UDP_SEGMENT, GRO, recvmmsg) is the low-effort path. `io-uring` (the raw crate) suits multishot. tokio-uring, glommio, monoio and compio add a runtime you don't need for a thread-per-lane design [speculative]. AF_XDP: `xsk-rs` or `aya` (eBPF loader) [not version-checked].
- Buffers (zero-copy slicing) [speculative design; the pre-sized pool pattern follows spead2's docs: "Using a memory pool is the single most important tool" [proven, spead2.readthedocs.io/en/latest/perf.html]]:
  - Intake: read TCP into a fixed pool of 64 KB slabs (BytesMut, or a pre-allocated arena; on Windows, the RIO-registered buffer).
  - Carve each slab into k segments of [seq u64 | len | payload], with the header room reserved at write time so there is no second copy. Hand each slab to lane (seq mod N) as one USO send.
  - Receive: a ring indexed by seq mod R, with R ≥ 2× (the product of max reorder depth and N). The consumer drains contiguous seqs and declares gaps lost after a timeout, since there is no retransmit.
  - Water levels: watch slab-pool free count and ring occupancy. Apply backpressure by not reading the TCP socket.
- Batching: 64 KB per send call (Linux UDP_SEGMENT max 64 segments [proven, Cloudflare]), and 16–64 msgs per recvmmsg. Tune from measurement. [speculative starting points]

## 4. Measurement, layer by layer
1. **Loopback intake:** a SCPI-simulator writer to the mux reader, discarding the data. Record GB/s and CPU per core.
2. **UDP send, Windows:** the mux with a synthetic source, send-only to the Linux box. Compare against multi-thread iperf3 `-u -b 0 -l 1472/8972`.
3. **Wire/NIC:** iperf3 TCP and UDP multi-stream baseline, with and without jumbo and USO.
4. **Reassembly, Linux:** a replayed pcap or a local generator into the reassembler alone (divan for the ring).
5. **Counters to watch:**
   - Windows: `Get-NetAdapterStatistics` (OutboundDiscardedPackets, ReceivedDiscardedPackets); perfmon UDPv4 "Datagrams No Port/sec", "Datagrams Received Errors/sec" and "Datagrams Sent/sec"; Network Interface "Packets Outbound Discarded", "Output Queue Length"; Processor "% Interrupt Time"/"% DPC Time" per core; `Get-NetAdapterRss`, `Get-NetAdapterUso`.
   - Linux: `ethtool -S` (rx_missed/rx_no_buffer, per-queue packets); `/proc/net/snmp` Udp InErrors, RcvbufErrors; `/proc/net/softnet_stat` (dropped, time_squeeze); `nstat -az | grep -i udp`; `/proc/interrupts` per queue; `mpstat -P ALL` for %soft.
   Counter names are from Scout's knowledge and were not re-checked [speculative].
