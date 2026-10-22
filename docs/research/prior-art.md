---
id: note:repo-evals/2026-10-21-socket-mux-prior-art
title: socket-mux prior art (FOSS projects) and reading list
created: 2026-10-21T17:15:00-07:00
updated: 2026-10-21T17:15:00-07:00
tags: [repo-eval, performance, dev-tools]
status: active
related: project:socket-mux, [max bandwidth](max-bandwidth.md)
sources: [sources/](sources/) ([excerpts.md](sources/excerpts.md), [repos.json](sources/repos.json), [index.json](sources/index.json))
---

# socket-mux prior art and reading (2026-10-21)

**Asked via:** the MUX DMUX bot, 16:51 PT, as an addition to the bandwidth note.
**How the repos were checked:** the unauthenticated GitHub REST API (license, language, last push, archived) at about 17:10 PT, and `curl` for non-GitHub URLs. "Pushed" is the last push to any branch, not a release. Licenses are as the API reports them; NOASSERTION means GitHub could not classify the license.
**Tags:** [proven] / [vendor-claimed] / [speculative], as in the sibling note ([max-bandwidth.md](max-bandwidth.md)).

## Projects, ranked by what socket-mux can take

**1. spead2.** https://github.com/ska-sa/spead2
- Facts: LGPL-3.0, C++/Python. Pushed 2026-10-19; release v4.5.0 on 2026-09-07. Linux (ibverbs on ConnectX) [V API].
- Take:
  - its heap/packet header with item pointers and offsets (a model for the seq header)
  - pre-sized memory pools
  - its `max_heaps` reorder window
  - its jumbo-frame guidance
  - core pinning
- Its docs say a memory pool is "the single most important tool" and advise against heaps under 16 KiB [proven docs]. It is a radio-astronomy DAQ transport, the closest match to the instrument case [speculative].

**2. msquic.** https://github.com/microsoft/msquic
- Facts: MIT, C. Pushed 2026-10-21; release v2.6.3 on 2026-10-13. Windows, Linux, macOS [V API].
- Take: its Windows datapath code for USO, URO and RSS-aligned per-core workers [vendor-claimed blog], and the secnetperf methodology.
- Its RIO datapath was removed (PR 5328) [vendor-claimed]. Its XDP receive is slow (issue 5980) [vendor-claimed].

**3. quinn / quinn-udp.** https://github.com/quinn-rs/quinn
- Facts: Apache-2.0, Rust. Pushed 2026-10-21. Windows, Linux, macOS [V API].
- Take: use `quinn-udp` directly for GSO/GRO, USO/URO and ECN cmsg handling. URO is off by default on Windows (PR 2092) [proven].
- Multipath QUIC exists only as an IETF draft. Multipath support in quinn and quiche was not checked [speculative/unchecked].

**4. kcp / kcp-go.**
- Repos: https://github.com/skywind3000/kcp (MIT, C, pushed 2026-06-23) and https://github.com/xtaci/kcp-go (MIT, Go, pushed 2026-10-21) [V API]. Cross-platform.
- Take: the sequence number and receive-window design (sn/una), and its FEC option (kcp-go) for no-retransmit loss repair [speculative fit].
- https://github.com/xtaci/kcptun returned **404** from the API at 17:10 PT, so it was not verified.

**5. xdp-tools / AF_XDP.**
- Repos:
  - https://github.com/xdp-project/xdp-tools (C, NOASSERTION, pushed 2026-10-20)
  - Rust: https://github.com/DouglasGray/xsk-rs (MIT, pushed 2026-09-08) and https://github.com/aya-rs/aya (Apache-2.0, pushed 2026-10-21)
  - Windows counterpart: https://github.com/microsoft/xdp-for-windows (MIT, pushed 2026-10-21) [V API]
- Use it only if the socket path falls short. These are Linux only, apart from XDP for Windows [proven]. **NIC/driver dependent.**

**6. DPDK.** https://github.com/DPDK/dpdk
- Facts: C. The API reports no license (DPDK is BSD-3 per Scout's knowledge [U]). Pushed 2026-10-21 [V API].
- Kernel bypass is overkill for 10G per the sources above [speculative]. **NIC dependent.**

**7. MPTCP.** https://github.com/multipath-tcp/mptcp_net-next
- Facts: C, pushed 2026-10-21 [V API]. Linux upstream since 5.6 [U].
- It is TCP, so not a direct fit. Its scheduler and subflow design are worth reading for lane striping [speculative]. Windows has no native MPTCP [U].

**8. UDT.**
- Sources: https://sourceforge.net/projects/udt/ (HTTP 200) and the fork https://github.com/securesocketfunneling/udt (C++, last push 2018-11-18) [V].
- It is unmaintained. Read it only for its rate-based congestion control and loss lists [speculative].

**9. Tsunami UDP.** https://sourceforge.net/projects/tsunami-udp/ (HTTP 200) [V]
- An old FASP-like bulk transfer: a TCP control channel plus a UDP data channel, with retransmit requests. Maintenance status was not checked [U].

**10. UFTP.** https://uftp-multicast.sourceforge.net/ (HTTP 200) [V]
- Multicast file transfer with NAK-based loss accounting. License and maintenance were not checked [U].

**11. bbcp.**
- Sources: https://www.slac.stanford.edu/~abh/bbcp/ (**timed out**, unverified) and the mirror https://github.com/eeertekin/bbcp (GPL-3.0, C, pushed 2019-10-29) [V API].
- It runs parallel TCP streams, not UDP. Its idea of striping one file across N streams maps onto the N lanes [speculative].

**12. EPICS / CASPER.**
- EPICS pvAccess: https://github.com/epics-base/pvAccessCPP (C++, pushed 2026-10-21). It is control-system oriented, not a bulk path [speculative].
- CASPER: https://github.com/casper-astro/mlib_devel (VHDL, pushed 2026-08-20). It is FPGA-side; SPEAD is the CASPER wire format [U].

## Reading, ranked for the Win64 acquisition → Linux compute 10GbE case
1. MS Learn, USO (driver requirements, MSS divisibility, no loopback): https://learn.microsoft.com/en-us/windows-hardware/drivers/network/udp-segmentation-offload-uso- [proven]
2. MS Learn, IPPROTO_UDP options (UDP_SEND_MSG_SIZE, UDP_RECV_MAX_COALESCED_SIZE): https://learn.microsoft.com/en-us/windows/win32/winsock/ipproto-udp-socket-options [proven]
3. Linux scaling.txt (RSS/RPS/RFS/XPS): https://www.kernel.org/doc/Documentation/networking/scaling.txt (HTTP 200) [proven]
4. spead2 performance tuning: https://spead2.readthedocs.io/en/latest/perf.html [proven]
5. Cloudflare, Accelerating UDP packet transmission for QUIC (GSO, sendmmsg, 64-segment limit): https://blog.cloudflare.com/accelerating-udp-packet-transmission-for-quic/ [vendor-claimed]
6. Cloudflare, How to receive a million packets per second (RSS, pinning, SO_REUSEPORT): https://blog.cloudflare.com/how-to-receive-a-million-packets/ [vendor-claimed]
7. Jaeger et al., QUIC on the Highway (10G testbed; buffers ≥10x default): https://arxiv.org/pdf/2309.16395 [proven]
8. MS Q&A 268716 (RIO in practice, WFP callout cost, USO 2.3 Mpps): https://learn.microsoft.com/en-us/answers/questions/268716/has-winsock-registered-io-performance-degraded-sin [vendor-claimed/anecdotal]
9. msquic PR 5328 (RIO removed) + Balance Performance in MsQuic and XDP: https://github.com/microsoft/msquic/pull/5328 , https://techcommunity.microsoft.com/blog/networkingblog/balance-performance-in-msquic-and-xdp/3627665 [vendor-claimed]
10. Kernel NAPI docs (busy poll, defer_hard_irqs): https://docs.kernel.org/networking/napi.html [proven]
11. Tailscale, Surpassing 10Gb/s (UDP GSO/GRO): https://tailscale.com/blog/more-throughput [vendor-claimed]
12. AF_XDP / io_uring zcrx: https://www.kernel.org/doc/html/latest/networking/iou-zcrx.html and the AF_XDP i40e cover letter https://lists.openwall.net/netdev/2018/08/28/62 [proven docs / vendor-claimed perf]

**Not found:**
- a Cloudflare io_uring UDP post (no specific post located)
- quinn-udp design notes beyond the code and PR 2092 (only code-level comments located)
