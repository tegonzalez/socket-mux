---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-roadmap]; otherwise, do not modify."
---

# socket-mux Roadmap

## Purpose

Hold the prospective evolution of socket-mux, both TX and RX, for anyone deciding what to build next: every prospect, with its prospective requirements and acceptance criteria. It also holds each outcome wanted later, with its sequencing, and each design or interface choice that awaits a decision. Nothing in this roadmap is a decision on behavior or mechanism.

## Orientation

Each prospect has an `R-NN` identity. The disposition `completed`, `superseded` or `absorbed` marks a prospect whose decision moved to the [design](design.md) or [specification](spec.md), with a successor reference; `proposed` marks a prospective change or a choice awaiting a decision, labeled `Proposal` when it is an open choice; `wanted` marks an outcome whose mechanics are unspecified; `deferred` marks one held until a named condition. Nested `R-NN.S-NN` and `R-NN.AC-NN` identifiers name a prospect's prospective requirements and acceptance criteria. The throughput prospects draw on the research in [max bandwidth](research/max-bandwidth.md) and [prior art](research/prior-art.md); that research carries no authority, and its tags [proven], [vendor-claimed] and [speculative] stay at each claim. Goals owners are the outcomes in [goals](goals.md#goals). `unranked` means no priority is set, and `unseeded` means no Change has been seeded. An open question that the goals, specification, design and constraints do not answer is held here as a `Proposal` prospect until it is decided ([Authority Index](index.md#query-routes)).

## Horizon

| id     | outcome                                                                              | goals owner                                | horizon                             | priority                               | dependencies                                                 | readiness                                                                                                                                                                                         | prospective Change | disposition |
| ------ | ------------------------------------------------------------------------------------ | ------------------------------------------ | ----------------------------------- | -------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ | ----------- |
| `R-08` | Rust crate set for TX and RX                                                         | [G7](goals.md#goals)                       | closed                              | unranked                               | none                                                         | decided: design [Rust Crate Set](design.md#rust-crate-set)                                                                                                                                        | unseeded           | completed   |
| `R-09` | Round-robin slice assignment                                                         | [G1](goals.md#goals)                       | closed                              | unranked                               | none                                                         | decided: design [Round-Robin Slice Assignment](design.md#round-robin-slice-assignment); the specification carries it                                                                              | unseeded           | completed   |
| `R-10` | Datagram header layout                                                               | [G3](goals.md#goals)                       | closed                              | unranked                               | none                                                         | decided: design [Datagram Header Layout](design.md#datagram-header-layout); the specification carries it                                                                                          | unseeded           | completed   |
| `R-12` | Default ports                                                                        | [G5](goals.md#goals)                       | closed                              | unranked                               | none                                                         | decided: design [Default Ports](design.md#default-ports); the specification carries it                                                                                                            | unseeded           | completed   |
| `R-13` | RX skips and reports loss                                                            | [G3](goals.md#goals); [G4](goals.md#goals) | closed                              | unranked                               | `R-10`                                                       | decided: design [Output Under Loss](design.md#output-under-loss); the specification carries it                                                                                                    | unseeded           | completed   |
| `R-14` | RX delivers the stream over TCP                                                      | [G1](goals.md#goals); [G6](goals.md#goals) | closed                              | unranked                               | none                                                         | decided: design [RX Delivery Over TCP](design.md#rx-delivery-over-tcp); the specification carries it                                                                                              | unseeded           | completed   |
| `R-15` | `socket-mux` command line                                                            | [G2](goals.md#goals); [G5](goals.md#goals) | closed                              | unranked                               | `R-12`                                                       | decided: design [Command-Line Options](design.md#command-line-options); the specification carries it                                                                                              | unseeded           | completed   |
| `R-16` | `tx` input operand                                                                   | [G1](goals.md#goals)                       | closed                              | unranked                               | `R-15`                                                       | decided: design [TX Input Operand](design.md#tx-input-operand); the specification carries it                                                                                                      | unseeded           | completed   |
| `R-17` | `tx-pat` seed option                                                                 | [G6](goals.md#goals)                       | closed                              | unranked                               | `R-15`                                                       | decided: design [TX-PAT Seed Option](design.md#tx-pat-seed-option); the specification carries it                                                                                                  | unseeded           | completed   |
| `R-18` | Self-locating Adler pattern blocks and validator                                     | [G3](goals.md#goals); [G6](goals.md#goals) | closed                              | unranked                               | `R-17`                                                       | decided: design [Self-Locating Adler Pattern Blocks](design.md#self-locating-adler-pattern-blocks); the specification carries it                                                                  | unseeded           | completed   |
| `R-21` | Spelling of the inter-packet delay timeout                                           | [G3](goals.md#goals)                       | closed                              | unranked                               | `R-15`                                                       | decided: design [Inter-Packet Delay Timeout Spelling](design.md#inter-packet-delay-timeout-spelling); the specification carries it                                                                | unseeded           | completed   |
| `R-22` | `check-pat` pattern checker subcommand                                               | [G6](goals.md#goals)                       | closed                              | unranked                               | `R-18`                                                       | decided: design [Check-Pat Checker Subcommand](design.md#check-pat-checker-subcommand); the specification carries it                                                                              | unseeded           | completed   |
| `R-23` | Provisional cap on held datagrams                                                    | [G1](goals.md#goals); [G3](goals.md#goals) | closed                              | unranked                               | none                                                         | replaced by design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo), which sets the RX memory limit                                                                    | unseeded           | superseded  |
| `R-31` | Per-lane OS threads without an async runtime                                         | [G1](goals.md#goals)                       | closed                              | unranked                               | `R-08`                                                       | absorbed into design [Thread-Per-Lane Blocking I/O](design.md#thread-per-lane-blocking-io); pinning continues as `R-43`                                                                           | unseeded           | absorbed    |
| `R-32` | quinn-udp for GSO, GRO and USO                                                       | [G1](goals.md#goals)                       | open                                | unranked                               | `R-08`; `R-39`                                               | research only; unmeasured                                                                                                                                                                         | unseeded           | proposed    |
| `R-33` | Win64 USO in 64 KB batches                                                           | [G1](goals.md#goals)                       | open                                | Win64 send rank 2 in research          | `R-32`; `R-10`; NIC and driver USO support                   | research only; unmeasured                                                                                                                                                                         | unseeded           | proposed    |
| `R-34` | Linux GRO receive with steered, pinned lanes                                         | [G1](goals.md#goals)                       | open                                | Linux receive ranks 1 to 3 in research | `R-32`; `R-37`; `R-43`; NIC n-tuple and multi-queue support  | per-lane receive threads are design; steering, GRO and ring sizing unmeasured; the measured RX adapter runs one receive queue with a 256-entry ring                                               | unseeded           | proposed    |
| `R-35` | Slab pool and sequence-indexed reassembly ring                                       | [G1](goals.md#goals); [G3](goals.md#goals) | closed                              | unranked                               | none                                                         | absorbed into design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo) and [No Heap Allocation In The Critical Path](design.md#no-heap-allocation-in-the-critical-path) | unseeded           | absorbed    |
| `R-36` | Jumbo MTU end to end                                                                 | [G1](goals.md#goals)                       | after the Windows peer              | Win64 send rank 3 in research          | `--datagram-size`; `R-42`; jumbo-capable switch and NIC path | waits for the Windows peer; the Linux path uses a 1500-byte MTU                                                                                                                                   | unseeded           | deferred    |
| `R-37` | Proposal: larger default RX `--udp-buffer`                                           | [G1](goals.md#goals)                       | open                                | unranked                               | `R-15`                                                       | measured: 16 MiB or more removes RX receive-buffer errors at about 9.4 Gbit/s; default value undecided                                                                                            | unseeded           | proposed    |
| `R-38` | RIO, io_uring zcrx and AF_XDP                                                        | [G1](goals.md#goals)                       | only if the socket path falls short | last in research                       | `R-39`                                                       | research only; NIC and driver dependent                                                                                                                                                           | unseeded           | deferred    |
| `R-39` | Per-layer throughput measurement                                                     | [G1](goals.md#goals)                       | open                                | unranked                               | a TX and RX binary                                           | two-host Linux 10GbE harness exists at `scripts/link-sweep`; Windows TX measurement awaits the peer                                                                                               | unseeded           | proposed    |
| `R-41` | Iterative optimization loop for latency and throughput                               | [G1](goals.md#goals)                       | after a stable MVP release          | unranked                               | a stable MVP release; relates to `R-39`                      | wanted; mechanics unspecified; waits on a stable MVP release                                                                                                                                      | unseeded           | wanted      |
| `R-42` | Windows x86_64 qualification; deferred Linux aarch64 and macOS current-build re-test | [G7](goals.md#goals)                       | Windows next; others deferred       | unranked                               | Windows peer; real-machine access for deferred targets       | Windows peer receives a cross-compiled binary, no build environment; macOS tested on an earlier build only                                                                                        | unseeded           | wanted      |
| `R-43` | Per-lane thread pinning                                                              | [G1](goals.md#goals)                       | open                                | Win64 send rank 1 in research          | `R-31`; `R-39`; NIC multi-queue support                      | thread-per-lane is design; pinning unmeasured                                                                                                                                                     | unseeded           | proposed    |
| `R-44` | Proposal: warn about or refuse a datagram size above the path MTU less 48            | [G1](goals.md#goals); [G7](goals.md#goals) | open                                | unranked                               | `R-15`; `R-36`                                               | the specification accepts sizes up to 65507 without warning; fragmentation loss is a known issue; choice undecided                                                                                | unseeded           | proposed    |
| `R-45` | Proposal: default lane count of 2                                                    | [G1](goals.md#goals); [G5](goals.md#goals) | open                                | unranked                               | `R-15`                                                       | measured: one lane reaches about 5.8 Gbit/s and two or more about 9.4 Gbit/s on 10GbE; the specification's default is 1; choice undecided                                                         | unseeded           | proposed    |

## Prospect definitions

### R-08 Completed: Rust Crate Set

- Disposition: completed. The decision is design [Rust Crate Set](design.md#rust-crate-set).

### R-09 Completed: Round-Robin Slice Assignment

- Disposition: completed. The [specification](spec.md#semantics) carries it, and the decision is design [Round-Robin Slice Assignment](design.md#round-robin-slice-assignment).

### R-10 Completed: Datagram Header Layout

- Disposition: completed. The [specification](spec.md#datagram-header) carries the fixed 24-byte header, and the decisions are design [Datagram Header Layout](design.md#datagram-header-layout) and [Session Identity And Restart](design.md#session-identity-and-restart).

### R-12 Completed: Default Ports

- Disposition: completed. The [specification](spec.md#public-surface) carries it, and the decision is design [Default Ports](design.md#default-ports).

### R-13 Completed: Output Under Loss

- Disposition: completed. The [specification](spec.md#semantics) carries it, and the decisions are design [Output Under Loss](design.md#output-under-loss) and [Loss And Summary Reporter](design.md#loss-and-summary-reporter).

### R-14 Completed: RX Delivery Over TCP

- Disposition: completed. The [specification](spec.md#public-surface) carries it, and the decision is design [RX Delivery Over TCP](design.md#rx-delivery-over-tcp).

### R-15 Completed: Command-Line Options

- Disposition: completed. The [specification](spec.md#command-line) carries it, and the decision is design [Command-Line Options](design.md#command-line-options).

### R-16 Completed: TX Input Operand

- Disposition: completed. The [specification](spec.md#tx-options) carries it, and the decision is design [TX Input Operand](design.md#tx-input-operand).

### R-17 Completed: TX-PAT Seed Option

- Disposition: completed. The [specification](spec.md#tx-pat-options) carries it, and the decision is design [TX-PAT Seed Option](design.md#tx-pat-seed-option).

### R-18 Completed: Self-Locating Adler Pattern Blocks

- Disposition: completed. The [specification](spec.md#semantics) carries it, and the decision is design [Self-Locating Adler Pattern Blocks](design.md#self-locating-adler-pattern-blocks).

### R-21 Completed: Inter-Packet Delay Timeout Spelling

- Disposition: completed. The [specification](spec.md#global-options) carries it, and the decision is design [Inter-Packet Delay Timeout Spelling](design.md#inter-packet-delay-timeout-spelling).

### R-22 Completed: check-pat Checker Subcommand

- Disposition: completed. The [specification](spec.md#command-line) carries it, and the decision is design [Check-Pat Checker Subcommand](design.md#check-pat-checker-subcommand).

### R-23 Superseded: Provisional Cap On Held Datagrams

- Disposition: superseded. Design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo) sets the RX memory limit that bounds the datagrams RX holds, under the loss limits in the [specification](spec.md#semantics).

### R-31 Absorbed: Per-Lane OS Threads

- Disposition: absorbed. One blocking thread per lane with no async runtime on TX and RX is design [Thread-Per-Lane Blocking I/O](design.md#thread-per-lane-blocking-io), with [Rust Crate Set](design.md#rust-crate-set) removing the runtime entirely. Pinning each lane thread to a core is not decided and continues as `R-43`.

### R-32 quinn-udp For GSO, GRO And USO

- User need: batched UDP I/O on both platforms without writing each platform's batching by hand ([prior art](research/prior-art.md), item 3).
- `R-32.S-01`: TX and RX use `quinn-udp` directly for GSO (UDP_SEGMENT), GRO and recvmmsg on Linux and for USO (UDP_SEND_MSG_SIZE) on Windows [proven, docs.rs].
- Assumption: URO (UDP_RECV_MAX_COALESCED_SIZE) stays off, as quinn-udp leaves it off by default on Windows after failures with WSL installed; it needs Windows 11 24H2 and NIC support [proven, quinn PR 2092].
- Prospective acceptance `R-32.AC-01`: the batched path builds on Linux, macOS and Windows ([Cross-Platform Dependencies](design.md#cross-platform-dependencies)).
- Applicability: adds quinn-udp to the `R-08` crate set as the batched I/O path inside the per-lane threads.
- Open question: whether quinn-udp alone covers the `R-33` buffer layout.

### R-33 Win64 USO 64 KB Batches

- User need: Windows TX sends at 10G+ without one syscall per datagram.
- `R-33.S-01`: TX calls `WSASetUdpSendMessageSize` (UDP_SEND_MSG_SIZE) and sends 64 KB buffers of MSS-sized segments.
- `R-33.S-02`: each segment is its own datagram carrying its own header (`R-10`) inside its MSS bytes, so the buffer is laid out as [hdr|payload]×k; unless the driver has SubMssFinalSegmentSupported, the buffer length is a multiple of the MSS [proven, MS docs].
- Platform: Windows 10 2004 or Server 2004 (build 19041) or later, and NIC and driver USO support checked with `Get-NetAdapterUso` and turned on with `Enable-NetAdapterUso` [proven, MS docs].
- Evidence: one answer reports 2.3 Mpps with UDP_SEND_MSG_SIZE on 64 KB buffers [vendor-claimed]; no independent Windows USO throughput number was found.
- Prospective acceptance `R-33.AC-01`: UDP send measured from Windows to Linux (`R-39.S-02`) with USO on exceeds the same run with USO off.
- Applicability: USO is not applied to loopback [proven], so a loopback back-to-back loop cannot show it.
- Open question: which NIC and driver the Windows peer has.

### R-34 Linux GRO Receive And Lane Steering

- User need: Linux RX receives every lane at line rate without receive-buffer or adapter drops.
- `R-34.S-01`: `ethtool -N` n-tuple rules steer dst-port base+k to queue k, IRQ affinity puts queue k on core k, and lane k's receive thread runs on core k (`R-43`).
- `R-34.S-02`: each lane socket sets UDP_GRO and reads with recvmmsg, splitting each coalesced read by the `UDP_GRO` cmsg gso_size; TX keeps all segments equal-size so GRO coalesces them [proven behavior; gain vendor-claimed].
- `R-34.S-03`: `ethtool -G rx` at the adapter's maximum ring size, with the RX receive buffer of `R-37`; buffer size is the top cause of drops (UdpRcvbufErrors) [proven, TUM paper].
- Evidence: on the measured 10GbE link the RX adapter runs one receive queue with a 256-entry ring, and the rare residual loss at about 9.4 Gbit/s is receive-ring misses in that adapter, which `--ipd-ns` pacing does not remove ([design](design.md#measured-link-behavior), [known issues](known-issues.md#register)).
- Assumptions: SO_REUSEPORT and BPF steering are not needed because each lane has its own port [speculative]; without n-tuple support RSS hashes the 4-tuple and lanes may share a queue, which `ethtool -S` shows [speculative].
- Prospective acceptance `R-34.AC-01`: during a measured run (`R-39.S-06`), per-queue packets in `ethtool -S` show lane k on queue k, and Udp RcvbufErrors in `/proc/net/snmp` and the adapter's receive-missed counter stay at zero.
- Applicability: needs NIC n-tuple and multi-queue support, which is common on 10G+. Kernel busy polling (`SO_BUSY_POLL`, `napi_defer_hard_irqs`) is outside the design [Wait Policy](design.md#wait-policy).
- Open question: io_uring multishot recvmsg has no UDP number against recvmmsg with GRO.

### R-35 Absorbed: Slab Pool And Reassembly Ring

- Disposition: absorbed. The fixed pools are design [No Heap Allocation In The Critical Path](design.md#no-heap-allocation-in-the-critical-path), and the sequence-indexed ring is the wheel of design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo).

### R-36 Jumbo MTU End To End

- User need: fewer datagrams per second for the same rate.
- `R-36.S-01`: MTU 9000 on both ends and on every hop between them.
- `R-36.S-02`: `--datagram-size` at most the path MTU less 48 bytes (the 40-byte IPv6 and 8-byte UDP headers), which is 8952 at MTU 9000; a larger datagram fragments and loses data ([design](design.md#measured-link-behavior)).
- Rationale: 10 Gbit/s is about 830k datagrams/s at 1472 B payload and about 140k/s at 9000 B, about 6× fewer [arithmetic; gain speculative]; spead2 gives jumbo-frame guidance ([prior art](research/prior-art.md), item 1).
- Prospective acceptance `R-36.AC-01`: the wire baseline (`R-39.S-03`) runs with and without jumbo frames on the target path.
- Applicability: needs a switch and NIC path that carries jumbo frames; `scripts/admin/sm-mtu.sh` prints the commands an administrator runs.
- Readiness: deferred until the Windows peer exists (`R-42`); the available Linux hosts use a 1500-byte MTU.

### R-37 Proposal: Larger Default RX UDP Buffer

- User need: RX runs without socket-buffer loss at its defaults.
- `R-37.S-01`: the default of `--udp-buffer` on RX is larger than the operating-system default that the specification's [Global Options](spec.md#global-options) give it; the TX default stays the operating-system default, because a large TX send buffer causes sender-side drops ([design](design.md#udp-socket-buffer-sizing)).
- Evidence: on the measured 10GbE link at about 9.4 Gbit/s, an RX `--udp-buffer` of 16 MiB or more removes UDP receive-buffer errors and 4 MiB does not; 64 MiB showed no loss in repeated two-lane runs ([design](design.md#measured-link-behavior)). QUIC measurements on 10G links found default buffers too small by 10× or more [proven, TUM paper].
- Prospective acceptance `R-37.AC-01`: lossless two-lane runs at about 9.4 Gbit/s on 10GbE pass with the default RX `--udp-buffer`, with no UDP receive-buffer errors.
- Applicability: Linux caps an RX request at `net.core.rmem_max`, 4194304 bytes by default on the measured hosts until an administrator raises it, and the capped request is reported ([specification](spec.md#diagnostics)); a larger default alone cannot raise the cap.
- Open question for decision: the RX default request. Options: 16 MiB, the smallest value measured to remove receive-buffer errors; 64 MiB, the value with no loss in repeated runs; or the operating-system default unchanged. Either larger value produces the capped-request warning on every RX start on a host whose cap was not raised.

### R-38 Deferred Kernel-Bypass Paths

- User need: headroom past the socket path, if it falls short.
- `R-38.S-01`: socket-mux defers RIO, io_uring zcrx and AF_XDP until measurement shows the socket path short of line rate.
- RIO (registered buffers and completion queues, through the windows-sys RIO function table) ranks last: its 2012 gains were about 1.3× to 2.5× [proven, old], a newer report found no gain [proven, anecdotal], msquic removed its RIO datapath [vendor-claimed], and no maintained RIO crate was found.
- io_uring zcrx (Linux 6.15+) is TCP-oriented and supports only specific Broadcom and Mellanox NICs; UDP support was not found [proven docs, vendor-claimed perf].
- AF_XDP zero-copy needs driver support (i40e yes; ice and mlx5 depend on configuration) and bypasses the UDP stack, so socket-mux would parse headers itself [proven]; Rust options are `xsk-rs` or `aya` [not version-checked].
- Applicability: all three are NIC or driver dependent; AF_XDP is Linux only apart from XDP for Windows, whose receive is reported slow [vendor-claimed].
- Open question: whether per-layer measurement (`R-39`) shows the socket path short of line rate.

### R-39 Per-Layer Measurement Plan

- User need: each throughput prospect is judged by measurement at its own layer.
- `R-39.S-01` loopback intake: a SCPI-simulator writer feeds the mux reader, which discards the data; record GB/s and CPU per core.
- `R-39.S-02` UDP send on Windows: the mux with a synthetic source sends only, to the Linux box; compare with a multi-threaded UDP sender at standard and jumbo datagram sizes.
- `R-39.S-03` wire and NIC: a TCP and UDP multi-stream baseline, with and without jumbo frames and USO.
- `R-39.S-04` reassembly on Linux: a replayed pcap or a local generator feeds the reassembler alone.
- `R-39.S-05` counters on Windows: adapter discards, UDP datagrams sent, received with errors and with no port, interface output discards and queue length, per-core interrupt and DPC time, and the adapter's RSS and USO state.
- `R-39.S-06` counters on Linux: NIC drops and per-queue packets, UDP receive and buffer errors, softnet drops and squeezes, per-queue interrupts and per-core softirq time.
- Assumption: the counter names in [max bandwidth](research/max-bandwidth.md) were not re-checked [speculative]; batching starting points are 64 KB per send call (UDP_SEGMENT allows at most 64 segments [proven, Cloudflare]) and 16 to 64 messages per recvmmsg, tuned from measurement [speculative].
- Prospective acceptance `R-39.AC-01`: each layer reports its rate and the counters above for the same run.
- Readiness: the two-host harness [`scripts/link-sweep`](../scripts/link-sweep/README.md) sweeps lanes, RX `--udp-buffer`, `--ipd-ns` and `--datagram-size` with `tx-pat --bytes` runs and records UDP, adapter, softnet and sender queueing-discipline counters on each host's local disk; its Linux 10GbE results are summarised in the [design](design.md#measured-link-behavior). The same sweep from Windows TX to Linux RX waits for the headless Windows x86_64 peer, which receives the cross-compiled `win64-x86_64` binary and needs no build environment. The performance goal is maximum throughput from Windows TX to Linux RX.
- Open question: the Windows peer's NIC capabilities; the platform pass criterion is defined in the [specification](spec.md#boundaries).

### R-41 Wanted: Iterative Optimization Loop For Latency And Throughput

- User need: an iterative optimization loop for latency and throughput, wanted after a stable MVP release.
- Outcome: an iterative optimization loop that improves socket-mux latency and throughput.
- Sequencing: the loop follows a stable MVP release.
- Relation: the per-layer measurement plan `R-39` names the rates and counters per layer that such a loop could measure.
- Unspecified: the loop's mechanics, its latency and throughput metrics and targets, and stability criteria beyond the [first-release platform pass criterion](spec.md#boundaries). No optimization-loop requirement or acceptance criterion is drafted.

### R-42 Wanted: Linux aarch64, Windows 10/11 And macOS Qualification

- User need: qualify Windows x86_64 next and retain deferred Linux aarch64 qualification and macOS re-testing of the current build. Linux x86_64 qualification of the current baseline is required first ([specification](spec.md#boundaries)).
- Outcome: Linux aarch64, Windows 10/11 x86_64 and macOS pass real-machine file-transfer and pattern-run qualification.
- Sequencing: Windows x86_64 qualification is next, pending a headless peer that receives the cross-compiled `win64-x86_64` binary, has no build environment and acts as TX to Linux RX. It does not depend on a release. Linux aarch64 is built but not tested, with qualification deferred. macOS was tested on an earlier build; re-testing the current build is deferred.
- Prospective acceptance `R-42.AC-01`: on real machines running Linux aarch64, Windows 10, Windows 11 and macOS, both a file transfer and a `tx-pat`/`check-pat` run pass on each target.
- Unspecified: macOS version and architecture coverage; the earlier test records neither.

### R-43 Per-Lane Thread Pinning

- User need: each lane's send and receive threads run on the core that services that lane's network queue, so lanes neither migrate nor contend for one core [speculative synthesis, [max bandwidth](research/max-bandwidth.md)].
- `R-43.S-01`: each TX lane send thread and each RX lane receive thread is pinned to a configured or derived core (`core_affinity`, or `SetThreadAffinityMask` and `sched_setaffinity` through windows-sys and libc).
- Rationale: one core is what caps single-stream iperf3, so thread-per-lane beats one runtime thread [speculative]; single-stream iperf3 reaches about 3 Gbps and multi-thread iperf3 gets near line rate. Unpinned per-lane threads already reach about 9.4 Gbit/s at two lanes on 10GbE ([design](design.md#measured-link-behavior)).
- Prospective acceptance `R-43.AC-01`: per-layer measurement (`R-39`) shows pinned lanes raise goodput or remove loss against the same unpinned run.
- Applicability: needs one adapter queue per lane to matter on receive (`R-34`); the measured RX adapter has one receive queue.
- Open question: whether pinning gains anything on a single-queue adapter, and how operators choose cores.

### R-44 Proposal: Datagram Size Above The Path MTU

- User need: an operator who sets `--datagram-size` above the path MTU less 48 bytes learns of the fragmentation loss it causes.
- `R-44.S-01`: when `--datagram-size` exceeds the local interface MTU less 48 bytes, TX and RX either warn or refuse.
- Evidence: datagrams above the MTU less 48 bytes fragment and lose data on the measured link ([design](design.md#measured-link-behavior)); the [specification](spec.md#semantics) accepts sizes up to 65507 without a warning, and the resulting loss is a [known issue](known-issues.md#register).
- Prospective acceptance `R-44.AC-01`: a run with a datagram size above the interface MTU less 48 produces the selected warning or usage error, and a run at or below it produces neither.
- Open question for decision: warn, refuse, or keep the current silent acceptance. Refusing blocks deliberate fragmentation runs such as the datagram-size sweep of `scripts/link-sweep`; warning or refusing needs a per-platform interface-MTU lookup, and the local interface MTU does not reveal a smaller MTU elsewhere on the path.

### R-45 Proposal: Default Lane Count

- User need: an operator who runs both ends at their defaults gets near line rate on the measured 10GbE link.
- `R-45.S-01`: the default of `--lanes` is 2 instead of the specification's 1 ([Global Options](spec.md#global-options)); the default stays configurable as [G5](goals.md#goals) requires.
- Evidence: on the measured 10GbE link one lane reaches about 5.8 Gbit/s, bounded by one sending thread, and two or more lanes reach about 9.4 Gbit/s ([design](design.md#measured-link-behavior)).
- Prospective acceptance `R-45.AC-01`: a two-host run with default `--lanes` on both ends reaches about 9.4 Gbit/s on 10GbE.
- Applicability: both ends must change together because there is no negotiation; an RX at default 2 listens on base port + 1, which an older TX at default 1 never sends to, and an older RX at default 1 never hears lane 1 of a newer TX.
- Open question for decision: keep 1, the smallest configuration and one port; or 2, the measured line-rate configuration with a second port and thread on each end.

## Dependencies

`R-08` records the crate set without an async runtime; `R-31` is absorbed into the thread-per-lane design and `R-43` continues its pinning part. `R-32` would add quinn-udp as the batched path that `R-33` and `R-34` use. `R-33` puts one `R-10` header in each USO segment. `R-34` needs equal-size segments from TX, the receive buffers of `R-37`, the pinning of `R-43` and a multi-queue adapter. `R-36` bears on the default `--datagram-size` and on `R-44`. `R-37`, `R-44` and `R-45` change defaults or checks that design [Command-Line Options](design.md#command-line-options) sets. `R-18` uses the seed of design [TX-PAT Seed Option](design.md#tx-pat-seed-option) and validates the output that design [Output Under Loss](design.md#output-under-loss) describes. `R-22` is the validator that `R-18` describes. `R-41` waits on a stable MVP release and relates to `R-39`. `R-42` puts Windows x86_64 qualification next, pending its peer, while Linux aarch64 qualification and macOS re-testing of the current build remain deferred. `R-36` waits on the Windows peer. `R-38` waits on `R-39`, and `R-32` to `R-37` and `R-43` are tuned from it.

## Readiness

A prospect is ready for a Change when its behavior is in the [specification](spec.md) and its mechanism is in the [design](design.md). A `Proposal` prospect is ready once its open question for decision is decided and the decision is written into its owner. Readiness does not activate work.

## Boundaries

Outcomes are in the [goals](goals.md), current behavior in the [specification](spec.md), current mechanism and decisions in the [design](design.md), forbidden implementation patterns in the [constraints](constraints.md), finite residuals in [known issues](known-issues.md), and background research in [research](research/). A prospect whose decision enters its owner retires here with a successor reference, and this roadmap records its disposition. This roadmap records no execution or release history. Retransmission is outside the horizon at this stage.
