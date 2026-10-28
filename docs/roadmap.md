---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-roadmap]; otherwise, do not modify."
---

# socket-mux Roadmap

## Purpose

Hold the prospective evolution of socket-mux, both TX and RX, for anyone deciding what to build next: every prospect, with its prospective requirements and acceptance criteria. It also holds each outcome wanted later, with its sequencing. Nothing in this roadmap is a decision on behavior or mechanism.

## Orientation

Each `SM-D` and `SM-R` prospect is a prospective change; the disposition `superseded` or `absorbed` marks one that the [design](design.md) replaced or took in, and `completed` marks one whose decision moved to the design and whose behavior the [specification](spec.md) carries. Each `SM-W` prospect is a wanted outcome, and its mechanics are unspecified. The `SM-D` prospects keep the identities that the [specification](spec.md) cites. The `SM-R` prospects are throughput prospects from the research in [max bandwidth](research/max-bandwidth.md) and [prior art](research/prior-art.md); that research carries no authority, and its tags [proven], [vendor-claimed] and [speculative] stay at each claim. `REQ` and `AC` identifiers name a prospect's prospective requirements and acceptance criteria. Goals owners are the outcomes in [goals](goals.md#goals). `unranked` means no priority is set, and `unseeded` means no Change has been seeded.

## Horizon

| id       | outcome                                                | goals owner                                | horizon                             | priority                               | dependencies                                           | readiness                                                                                                                                                                                         | prospective Change | disposition |
| -------- | ------------------------------------------------------ | ------------------------------------------ | ----------------------------------- | -------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ | ----------- |
| `SM-D8`  | Rust crate set for TX and RX                           | [G7](goals.md#goals)                       | closed                              | unranked                               | none                                                   | decided: design [Rust Crate Set](design.md#rust-crate-set)                                                                                                                                        | unseeded           | completed   |
| `SM-D9`  | Round-robin slice assignment                           | [G1](goals.md#goals)                       | closed                              | unranked                               | none                                                   | decided: design [Round-Robin Slice Assignment](design.md#round-robin-slice-assignment); the specification carries it                                                                              | unseeded           | completed   |
| `SM-D10` | Datagram header layout                                 | [G3](goals.md#goals)                       | closed                              | unranked                               | none                                                   | decided: design [Datagram Header Layout](design.md#datagram-header-layout); the specification carries it                                                                                          | unseeded           | completed   |
| `SM-D12` | Default ports                                          | [G5](goals.md#goals)                       | closed                              | unranked                               | none                                                   | decided: design [Default Ports](design.md#default-ports); the specification carries it                                                                                                            | unseeded           | completed   |
| `SM-D13` | RX skips and logs loss gaps                            | [G3](goals.md#goals); [G4](goals.md#goals) | closed                              | unranked                               | `SM-D10`                                               | decided: design [Output Under Loss](design.md#output-under-loss); the specification carries it                                                                                                    | unseeded           | completed   |
| `SM-D14` | RX delivers the stream over TCP                        | [G1](goals.md#goals); [G6](goals.md#goals) | closed                              | unranked                               | none                                                   | decided: design [RX Delivery Over TCP](design.md#rx-delivery-over-tcp); the specification carries it                                                                                              | unseeded           | completed   |
| `SM-D15` | `socket-mux` command line                              | [G2](goals.md#goals); [G5](goals.md#goals) | closed                              | unranked                               | `SM-D12`                                               | decided: design [Command-Line Options](design.md#command-line-options); the specification carries it                                                                                              | unseeded           | completed   |
| `SM-D16` | `tx` input operand                                     | [G1](goals.md#goals)                       | closed                              | unranked                               | `SM-D15`                                               | decided: design [TX Input Operand](design.md#tx-input-operand); the specification carries it                                                                                                      | unseeded           | completed   |
| `SM-D17` | `tx-pat` seed option                                   | [G6](goals.md#goals)                       | closed                              | unranked                               | `SM-D15`                                               | decided: design [TX-PAT Seed Option](design.md#tx-pat-seed-option); the specification carries it                                                                                                  | unseeded           | completed   |
| `SM-D18` | Self-locating Adler pattern blocks and validator       | [G3](goals.md#goals); [G6](goals.md#goals) | closed                              | unranked                               | `SM-D17`                                               | decided: design [Self-Locating Adler Pattern Blocks](design.md#self-locating-adler-pattern-blocks); the specification carries it                                                                  | unseeded           | completed   |
| `SM-D21` | Spelling of the inter-packet delay timeout             | [G3](goals.md#goals)                       | closed                              | unranked                               | `SM-D15`                                               | decided: design [Inter-Packet Delay Timeout Spelling](design.md#inter-packet-delay-timeout-spelling); the specification carries it                                                                | unseeded           | completed   |
| `SM-D22` | `check-pat` pattern checker subcommand                 | [G6](goals.md#goals)                       | closed                              | unranked                               | `SM-D18`                                               | decided: design [Check-Pat Checker Subcommand](design.md#check-pat-checker-subcommand); the specification carries it                                                                              | unseeded           | completed   |
| `SM-D23` | Provisional cap on held datagrams                      | [G1](goals.md#goals); [G3](goals.md#goals) | closed                              | unranked                               | none                                                   | replaced by design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo), which sets the RX memory limit                                                                    | unseeded           | superseded  |
| `SM-R1`  | Pinned OS thread per lane; tokio only for control      | [G1](goals.md#goals)                       | open                                | Win64 send rank 1 in research          | `SM-D8`; `SM-R9`                                       | research only; unmeasured                                                                                                                                                                         | unseeded           | proposed    |
| `SM-R2`  | quinn-udp for GSO, GRO and USO                         | [G1](goals.md#goals)                       | open                                | unranked                               | `SM-D8`; `SM-R9`                                       | research only; unmeasured                                                                                                                                                                         | unseeded           | proposed    |
| `SM-R3`  | Win64 USO in 64 KB batches                             | [G1](goals.md#goals)                       | open                                | Win64 send rank 2 in research          | `SM-R2`; `SM-D10`; NIC and driver USO support          | research only; unmeasured                                                                                                                                                                         | unseeded           | proposed    |
| `SM-R4`  | Linux GRO receive with steered, pinned lanes           | [G1](goals.md#goals)                       | open                                | Linux receive ranks 1 to 3 in research | `SM-R2`; `SM-R7`; NIC n-tuple support                  | research only; unmeasured                                                                                                                                                                         | unseeded           | proposed    |
| `SM-R5`  | Slab pool and sequence-indexed reassembly ring         | [G1](goals.md#goals); [G3](goals.md#goals) | closed                              | unranked                               | none                                                   | absorbed into design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo) and [No Heap Allocation In The Critical Path](design.md#no-heap-allocation-in-the-critical-path) | unseeded           | absorbed    |
| `SM-R6`  | Jumbo MTU end to end                                   | [G1](goals.md#goals)                       | open                                | Win64 send rank 3 in research          | `--datagram-size`; a jumbo-capable switch and NIC path | research only; unmeasured                                                                                                                                                                         | unseeded           | proposed    |
| `SM-R7`  | Larger default `--udp-buffer`                          | [G1](goals.md#goals)                       | open                                | Linux receive rank 3 in research       | `SM-D15`                                               | no value chosen                                                                                                                                                                                   | unseeded           | proposed    |
| `SM-R8`  | RIO, io_uring zcrx and AF_XDP                          | [G1](goals.md#goals)                       | only if the socket path falls short | last in research                       | `SM-R9`                                                | research only; NIC and driver dependent                                                                                                                                                           | unseeded           | deferred    |
| `SM-R9`  | Per-layer throughput measurement                       | [G1](goals.md#goals)                       | open                                | unranked                               | a TX and RX binary                                     | research only                                                                                                                                                                                     | unseeded           | proposed    |
| `SM-W1`  | Iterative optimization loop for latency and throughput | [G1](goals.md#goals)                       | after a stable MVP release          | unranked                               | a stable MVP release; relates to `SM-R9`               | wanted; mechanics unspecified; waits on a stable MVP release                                                                                                                                      | unseeded           | wanted      |
| `SM-W2`  | Windows 10/11 and macOS qualification                  | [G7](goals.md#goals)                       | after the first release             | unranked                               | the first release; a real test machine for each OS     | wanted; awaits post-MVP real-machine qualification                                                                                                                                                | unseeded           | wanted      |

## Prospect definitions

### SM-D8 Completed: Rust Crate Set

- Disposition: completed. The decision is design [Rust Crate Set](design.md#rust-crate-set).

### SM-D9 Completed: Round-Robin Slice Assignment

- Disposition: completed. The [specification](spec.md#semantics) carries it, and the decision is design [Round-Robin Slice Assignment](design.md#round-robin-slice-assignment).

### SM-D10 Completed: Datagram Header Layout

- Disposition: completed. The [specification](spec.md#datagram-header) carries the fixed 24-byte header, and the decisions are design [Datagram Header Layout](design.md#datagram-header-layout) and [Session Identity And Restart](design.md#session-identity-and-restart).

### SM-D12 Completed: Default Ports

- Disposition: completed. The [specification](spec.md#public-surface) carries it, and the decision is design [Default Ports](design.md#default-ports).

### SM-D13 Completed: Output Under Loss

- Disposition: completed. The [specification](spec.md#semantics) carries it, and the decision is design [Output Under Loss](design.md#output-under-loss).

### SM-D14 Completed: RX Delivery Over TCP

- Disposition: completed. The [specification](spec.md#public-surface) carries it, and the decision is design [RX Delivery Over TCP](design.md#rx-delivery-over-tcp).

### SM-D15 Completed: Command-Line Options

- Disposition: completed. The [specification](spec.md#command-line) carries it, and the decision is design [Command-Line Options](design.md#command-line-options).

### SM-D16 Completed: TX Input Operand

- Disposition: completed. The [specification](spec.md#tx-options) carries it, and the decision is design [TX Input Operand](design.md#tx-input-operand).

### SM-D17 Completed: TX-PAT Seed Option

- Disposition: completed. The [specification](spec.md#tx-pat-options) carries it, and the decision is design [TX-PAT Seed Option](design.md#tx-pat-seed-option).

### SM-D18 Completed: Self-Locating Adler Pattern Blocks

- Disposition: completed. The [specification](spec.md#semantics) carries it, and the decision is design [Self-Locating Adler Pattern Blocks](design.md#self-locating-adler-pattern-blocks).

### SM-D21 Completed: Inter-Packet Delay Timeout Spelling

- Disposition: completed. The [specification](spec.md#global-options) carries it, and the decision is design [Inter-Packet Delay Timeout Spelling](design.md#inter-packet-delay-timeout-spelling).

### SM-D22 Completed: check-pat Checker Subcommand

- Disposition: completed. The [specification](spec.md#command-line) carries it, and the decision is design [Check-Pat Checker Subcommand](design.md#check-pat-checker-subcommand).

### SM-D23 Superseded: Provisional Cap On Held Datagrams

- Disposition: superseded. Design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo) sets the RX memory limit that bounds the datagrams RX holds, under the loss limits in the [specification](spec.md#semantics).

### SM-R1 Pinned Per-Lane OS Threads

- User need: TX and RX move one stream at near line rate; per-packet syscall and CPU cost is the limit, not the wire [speculative synthesis, [max bandwidth](research/max-bandwidth.md)].
- `SM-R1-REQ1`: each lane runs on its own `std::thread`, pinned to the core that services its RSS queue (`core_affinity`, or SetThreadAffinityMask and sched_setaffinity through windows-sys and libc), with blocking or overlapped I/O and no async runtime on the hot path.
- `SM-R1-REQ2`: tokio, if used at all, runs only the control plane.
- Rationale: one core is what caps single-stream iperf3, so thread-per-lane beats one current_thread runtime [speculative]; single-stream iperf3 reaches about 3 Gbps and multi-thread iperf3 gets near line rate.
- Prospective acceptance `SM-R1-AC1`: per-layer measurement (`SM-R9`) shows the lane threads, not one runtime core, set the send and receive rate.
- Applicability: revises the tokio current_thread pick in `SM-D8` for the data path.
- Open question: thread-per-lane or the `SM-D8` runtime?

### SM-R2 quinn-udp For GSO, GRO And USO

- User need: batched UDP I/O on both platforms without writing each platform's batching by hand ([prior art](research/prior-art.md), item 3).
- `SM-R2-REQ1`: TX and RX use `quinn-udp` directly for GSO (UDP_SEGMENT), GRO and recvmmsg on Linux and for USO (UDP_SEND_MSG_SIZE) on Windows [proven, docs.rs].
- Assumption: URO (UDP_RECV_MAX_COALESCED_SIZE) stays off, as quinn-udp leaves it off by default on Windows after failures with WSL installed; it needs Windows 11 24H2 and NIC support [proven, quinn PR 2092].
- Prospective acceptance `SM-R2-AC1`: the batched path builds on Linux, macOS and Windows ([Cross-Platform Dependencies](design.md#cross-platform-dependencies)).
- Applicability: adds quinn-udp to the current `SM-D8` crate set as the batched I/O path.
- Open question: whether quinn-udp alone covers the `SM-R3` buffer layout.

### SM-R3 Win64 USO 64 KB Batches

- User need: Windows TX sends at 10G+ without one syscall per datagram.
- `SM-R3-REQ1`: TX calls `WSASetUdpSendMessageSize` (UDP_SEND_MSG_SIZE) and sends 64 KB buffers of MSS-sized segments.
- `SM-R3-REQ2`: each segment is its own datagram carrying its own header (`SM-D10`) inside its MSS bytes, so the buffer is laid out as [hdr|payload]×k; unless the driver has SubMssFinalSegmentSupported, the buffer length is a multiple of the MSS [proven, MS docs].
- Platform: Windows 10 2004 or Server 2004 (build 19041) or later, and NIC and driver USO support checked with `Get-NetAdapterUso` and turned on with `Enable-NetAdapterUso` [proven, MS docs].
- Evidence: one answer reports 2.3 Mpps with UDP_SEND_MSG_SIZE on 64 KB buffers [vendor-claimed]; no independent Windows USO throughput number was found.
- Prospective acceptance `SM-R3-AC1`: UDP send measured from Windows to Linux (`SM-R9` layer 2) with USO on exceeds the same run with USO off.
- Applicability: USO is not applied to loopback [proven], so a loopback back-to-back loop cannot show it.
- Open question: which NIC and driver the instrument PC has.

### SM-R4 Linux GRO Receive And Lane Steering

- User need: Linux RX receives every lane at line rate without receive-buffer drops.
- `SM-R4-REQ1`: one socket per lane port, each on its own thread; `ethtool -N` n-tuple rules steer dst-port base+k to queue k, IRQ affinity puts queue k on core k, and lane k's receiver thread runs on core k.
- `SM-R4-REQ2`: each lane socket sets UDP_GRO and reads with recvmmsg, splitting each coalesced read by the `UDP_GRO` cmsg gso_size; TX keeps all segments equal-size so GRO coalesces them [proven behavior; gain vendor-claimed].
- `SM-R4-REQ3`: SO_RCVBUF of tens of MB with `rmem_max` raised to allow it, and `ethtool -G rx` at the maximum; buffer size is the top cause of drops (UdpRcvbufErrors) [proven, TUM paper].
- Assumptions: SO_REUSEPORT and BPF steering are not needed because each lane has its own port [speculative]; without n-tuple support RSS hashes the 4-tuple and lanes may share a queue, which `ethtool -S` shows [speculative].
- Prospective acceptance `SM-R4-AC1`: during a measured run (`SM-R9` layer 5), per-queue packets in `ethtool -S` show lane k on queue k, and Udp RcvbufErrors in `/proc/net/snmp` stays at zero.
- Applicability: needs NIC n-tuple support, which is common on 10G+.
- Open questions: busy polling (`napi_defer_hard_irqs` with `gro_flush_timeout`, or SO_BUSY_POLL) trades CPU for fewer IRQs [proven, kernel docs] with unmeasured gain here; io_uring multishot recvmsg has no UDP number against recvmmsg with GRO.

### SM-R5 Absorbed: Slab Pool And Reassembly Ring

- Disposition: absorbed. The fixed pools are design [No Heap Allocation In The Critical Path](design.md#no-heap-allocation-in-the-critical-path), and the sequence-indexed ring is the wheel of design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo).

### SM-R6 Jumbo MTU End To End

- User need: fewer datagrams per second for the same rate.
- `SM-R6-REQ1`: MTU 9000 on both ends and on every hop between them.
- Rationale: 10 Gbit/s is about 830k datagrams/s at 1472 B payload and about 140k/s at 9000 B, about 6× fewer [arithmetic; gain speculative]; spead2 gives jumbo-frame guidance ([prior art](research/prior-art.md), item 1).
- Prospective acceptance `SM-R6-AC1`: the wire baseline (`SM-R9` layer 3) runs with and without jumbo frames on the target path.
- Applicability: needs a switch and NIC path that carries jumbo frames.
- Open question: the datagram size to use with jumbo frames.

### SM-R7 Larger Default UDP Buffer

- User need: TX and RX run without loss from socket buffers at their defaults.
- `SM-R7-REQ1`: the default of `--udp-buffer` is larger than the operating-system default that the specification's Global Options give it (`SM-D15`).
- Evidence: QUIC measurements on 10G links found default buffers too small by 10× or more [proven, TUM paper]; the research proposes SO_RCVBUF of tens of MB on Linux and SO_SNDBUF of several MB on Windows [speculative].
- Prospective acceptance `SM-R7-AC1`: lossless and relay runs pass with the default `--udp-buffer`, with no loss gap logged.
- Applicability: on Linux SO_RCVBUF is capped by `rmem_max` (`SM-R4-REQ3`).
- Open question: the default value.

### SM-R8 Deferred Kernel-Bypass Paths

- User need: headroom past the socket path, if it falls short.
- `SM-R8-REQ1`: socket-mux defers RIO, io_uring zcrx and AF_XDP until measurement shows the socket path short of line rate.
- RIO (registered buffers and completion queues, through the windows-sys RIO function table) ranks last: its 2012 gains were about 1.3× to 2.5× [proven, old], a newer report found no gain [proven, anecdotal], msquic removed its RIO datapath [vendor-claimed], and no maintained RIO crate was found.
- io_uring zcrx (Linux 6.15+) is TCP-oriented and supports only specific Broadcom and Mellanox NICs; UDP support was not found [proven docs, vendor-claimed perf].
- AF_XDP zero-copy needs driver support (i40e yes; ice and mlx5 depend on configuration) and bypasses the UDP stack, so socket-mux would parse headers itself [proven]; Rust options are `xsk-rs` or `aya` [not version-checked].
- Applicability: all three are NIC or driver dependent; AF_XDP is Linux only apart from XDP for Windows, whose receive is reported slow [vendor-claimed].
- Open question: whether per-layer measurement (`SM-R9`) shows the socket path short of line rate.

### SM-R9 Per-Layer Measurement Plan

- User need: each throughput prospect is judged by measurement at its own layer.
- `SM-R9-REQ1` loopback intake: a SCPI-simulator writer feeds the mux reader, which discards the data; record GB/s and CPU per core.
- `SM-R9-REQ2` UDP send on Windows: the mux with a synthetic source sends only, to the Linux box; compare with a multi-threaded UDP sender at standard and jumbo datagram sizes.
- `SM-R9-REQ3` wire and NIC: a TCP and UDP multi-stream baseline, with and without jumbo frames and USO.
- `SM-R9-REQ4` reassembly on Linux: a replayed pcap or a local generator feeds the reassembler alone.
- `SM-R9-REQ5` counters on Windows: adapter discards, UDP datagrams sent, received with errors and with no port, interface output discards and queue length, per-core interrupt and DPC time, and the adapter's RSS and USO state.
- `SM-R9-REQ6` counters on Linux: NIC drops and per-queue packets, UDP receive and buffer errors, softnet drops and squeezes, per-queue interrupts and per-core softirq time.
- Assumption: the counter names in [max bandwidth](research/max-bandwidth.md) were not re-checked [speculative]; batching starting points are 64 KB per send call (UDP_SEGMENT allows at most 64 segments [proven, Cloudflare]) and 16 to 64 messages per recvmmsg, tuned from measurement [speculative].
- Prospective acceptance `SM-R9-AC1`: each layer reports its rate and the counters above for the same run.
- Open question: the measurement hosts and NICs; the first-release OS/architecture targets and real-machine pass criterion are defined in the [specification](spec.md#boundaries).

### SM-W1 Wanted: Iterative Optimization Loop For Latency And Throughput

- User need: an iterative optimization loop for latency and throughput, wanted after a stable MVP release.
- Outcome: an iterative optimization loop that improves socket-mux latency and throughput.
- Sequencing: the loop follows a stable MVP release.
- Relation: the per-layer measurement plan `SM-R9` names the rates and counters per layer that such a loop could measure.
- Unspecified: the loop's mechanics, its latency and throughput metrics and targets, and stability criteria beyond the [first-release platform pass criterion](spec.md#boundaries). No optimization-loop requirement or acceptance criterion is drafted.

### SM-W2 Wanted: Windows 10/11 And macOS Qualification

- User need: qualify the Windows and macOS build targets for supported use beyond the Linux-only first release.
- Outcome: Windows 10/11 x86_64 and macOS pass real-machine file-transfer and pattern-run qualification.
- Sequencing: qualification follows the first release and requires a real test machine running each target OS.
- Prospective acceptance `SM-W2-AC1`: on real machines running Windows 10, Windows 11 and macOS, both a file transfer and a `tx-pat`/`check-pat` run pass on each OS.
- Unspecified: macOS version and architecture coverage.

## Dependencies

`SM-D8` records Rust and Tokio as the current crate set; `SM-R1` proposes a future per-lane-thread architecture that would change the data-path runtime. `SM-R2` would add quinn-udp as the batched path that `SM-R3` and `SM-R4` use. `SM-R3` puts one `SM-D10` header in each USO segment. `SM-R4` needs equal-size segments from TX and the receive buffers of `SM-R7`. `SM-R6` bears on the default `--datagram-size`. `SM-R7` changes a default that design [Command-Line Options](design.md#command-line-options) sets. `SM-D18` uses the seed of design [TX-PAT Seed Option](design.md#tx-pat-seed-option) and validates the output that design [Output Under Loss](design.md#output-under-loss) describes. `SM-D22` is the validator that `SM-D18` describes. `SM-W1` waits on a stable MVP release and relates to `SM-R9`. `SM-W2` waits on the first release and real-machine test access for Windows and macOS. `SM-R8` waits on `SM-R9`, and `SM-R1` to `SM-R7` are tuned from it.

## Readiness

A prospect is ready for a Change when its behavior is in the [specification](spec.md) and its mechanism is in the [design](design.md). Readiness does not activate work.

## Boundaries

Outcomes are in the [goals](goals.md), current behavior in the [specification](spec.md), current mechanism and decisions in the [design](design.md), and background research in [research](research/). A prospect whose decision enters its owner retires here with a successor reference, and this roadmap records its disposition. This roadmap records no execution or release history. Retransmission is outside the horizon at this stage.
