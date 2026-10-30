---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-roadmap]; otherwise, do not modify."
---

# socket-mux Roadmap

## Purpose

Hold the prospective evolution of socket-mux, both TX and RX, for anyone deciding what to build next: every admitted prospect, with its prospective requirements and acceptance criteria. It also holds each outcome wanted later, with its sequencing. Nothing in this roadmap is a decision on behavior or mechanism.

## Orientation

Each prospect has an `R-NN` identity, which is never reused. Every prospect that is not retired is future work admitted to this horizon, and its readiness reports which owners have admitted its content. The disposition `completed`, `superseded` or `absorbed` retires a prospect whose decision moved to the [design](design.md) or [specification](spec.md), with a successor reference; `admitted` marks a prospect whose requirements and acceptance criteria are defined at their current maturity; `wanted` marks an outcome whose mechanics are unspecified; `deferred` marks one held until a named condition. Nested `R-NN.S-NN` and `R-NN.AC-NN` identifiers name a prospect's prospective requirements and acceptance criteria. The throughput prospects draw on the research in [max bandwidth](research/max-bandwidth.md) and [prior art](research/prior-art.md); that research carries no authority, and its tags [proven], [vendor-claimed] and [speculative] stay at each claim. Goals owners are the outcomes in [goals](goals.md#goals). `unranked` means no priority is set, and `unseeded` means no Change has been seeded. A choice that the authority basis cannot answer is not a prospect; the [Authority Index](index.md#query-routes) routes it.

## Horizon

| id     | outcome                                                                              | goals owner                                | horizon                             | priority                                | dependencies                                                                     | readiness                                                                                                                                                                                                                    | prospective Change | disposition |
| ------ | ------------------------------------------------------------------------------------ | ------------------------------------------ | ----------------------------------- | --------------------------------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ | ----------- |
| `R-08` | Rust crate set for TX and RX                                                         | [G7](goals.md#goals)                       | closed                              | unranked                                | none                                                                             | decided: design [Rust Crate Set](design.md#rust-crate-set)                                                                                                                                                                   | unseeded           | completed   |
| `R-09` | Round-robin slice assignment                                                         | [G1](goals.md#goals)                       | closed                              | unranked                                | none                                                                             | decided: design [Round-Robin Slice Assignment](design.md#round-robin-slice-assignment); the specification carries it                                                                                                         | unseeded           | completed   |
| `R-10` | Datagram header layout                                                               | [G3](goals.md#goals)                       | closed                              | unranked                                | none                                                                             | decided: design [Datagram Header Layout](design.md#datagram-header-layout); the specification carries it                                                                                                                     | unseeded           | completed   |
| `R-12` | Default ports                                                                        | [G5](goals.md#goals)                       | closed                              | unranked                                | none                                                                             | decided: design [Default Ports](design.md#default-ports); the specification carries it                                                                                                                                       | unseeded           | completed   |
| `R-13` | RX skips and reports loss                                                            | [G3](goals.md#goals); [G4](goals.md#goals) | closed                              | unranked                                | `R-10`                                                                           | decided: design [Output Under Loss](design.md#output-under-loss); the specification carries it                                                                                                                               | unseeded           | completed   |
| `R-14` | RX delivers the stream over TCP                                                      | [G1](goals.md#goals); [G6](goals.md#goals) | closed                              | unranked                                | none                                                                             | decided: design [RX Delivery Over TCP](design.md#rx-delivery-over-tcp); the specification carries it                                                                                                                         | unseeded           | completed   |
| `R-15` | `socket-mux` command line                                                            | [G2](goals.md#goals); [G5](goals.md#goals) | closed                              | unranked                                | `R-12`                                                                           | decided: design [Command-Line Options](design.md#command-line-options); the specification carries it                                                                                                                         | unseeded           | completed   |
| `R-16` | `tx` input operand                                                                   | [G1](goals.md#goals)                       | closed                              | unranked                                | `R-15`                                                                           | decided: design [TX Input Operand](design.md#tx-input-operand); the specification carries it                                                                                                                                 | unseeded           | completed   |
| `R-17` | `tx-pat` seed option                                                                 | [G6](goals.md#goals)                       | closed                              | unranked                                | `R-15`                                                                           | decided: design [TX-PAT Seed Option](design.md#tx-pat-seed-option); the specification carries it                                                                                                                             | unseeded           | completed   |
| `R-18` | Self-locating Adler pattern blocks and validator                                     | [G3](goals.md#goals); [G6](goals.md#goals) | closed                              | unranked                                | `R-17`                                                                           | decided: design [Self-Locating Adler Pattern Blocks](design.md#self-locating-adler-pattern-blocks); the specification carries it                                                                                             | unseeded           | completed   |
| `R-21` | Spelling of the inter-packet delay timeout                                           | [G3](goals.md#goals)                       | closed                              | unranked                                | `R-15`                                                                           | decided: design [Inter-Packet Delay Timeout Spelling](design.md#inter-packet-delay-timeout-spelling); the specification carries it                                                                                           | unseeded           | completed   |
| `R-22` | `check-pat` pattern checker subcommand                                               | [G6](goals.md#goals)                       | closed                              | unranked                                | `R-18`                                                                           | decided: design [Check-Pat Checker Subcommand](design.md#check-pat-checker-subcommand); the specification carries it                                                                                                         | unseeded           | completed   |
| `R-23` | Provisional cap on held datagrams                                                    | [G1](goals.md#goals); [G3](goals.md#goals) | closed                              | unranked                                | none                                                                             | replaced by design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo), which sets the RX memory limit                                                                                               | unseeded           | superseded  |
| `R-31` | Per-lane OS threads without an async runtime                                         | [G1](goals.md#goals)                       | closed                              | unranked                                | `R-08`                                                                           | absorbed into design [Thread-Per-Lane Blocking I/O](design.md#thread-per-lane-blocking-io)                                                                                                                                   | unseeded           | absorbed    |
| `R-34` | Linux RX adapter receive tuning                                                      | [G1](goals.md#goals)                       | open                                | Linux receive ranks 1 and 3 in research | `R-39`; NIC n-tuple and multi-queue support; administrator access on the RX host | outcome admitted: [known issues](known-issues.md#register) route adapter-side tuning here; the measured RX adapter runs one receive queue with a 256-entry ring; ring sizing and steering unmeasured                         | unseeded           | admitted    |
| `R-35` | Slab pool and sequence-indexed reassembly ring                                       | [G1](goals.md#goals); [G3](goals.md#goals) | closed                              | unranked                                | none                                                                             | absorbed into design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo) and [No Heap Allocation In The Critical Path](design.md#no-heap-allocation-in-the-critical-path)                            | unseeded           | absorbed    |
| `R-36` | Jumbo MTU end to end                                                                 | [G1](goals.md#goals)                       | after the Windows peer              | Win64 send rank 3 in research           | `R-42`; jumbo-capable switch and NIC path                                        | waits for the Windows peer; the Linux path uses a 1500-byte MTU                                                                                                                                                              | unseeded           | deferred    |
| `R-38` | RIO, io_uring zcrx and AF_XDP                                                        | [G1](goals.md#goals)                       | only if the socket path falls short | last in research                        | `R-39`                                                                           | research only; NIC and driver dependent                                                                                                                                                                                      | unseeded           | deferred    |
| `R-39` | Per-layer throughput measurement                                                     | [G1](goals.md#goals)                       | open                                | unranked                                | a TX and RX binary                                                               | outcome admitted: the [design](design.md#deployment-and-test-topology) routes measurement readiness and sequencing here; two-host Linux 10GbE harness exists at `scripts/link-sweep`; Windows TX measurement awaits the peer | unseeded           | admitted    |
| `R-41` | Iterative optimization loop for latency and throughput                               | [G1](goals.md#goals)                       | after a stable MVP release          | unranked                                | a stable MVP release; relates to `R-39`                                          | wanted; mechanics unspecified; waits on a stable MVP release                                                                                                                                                                 | unseeded           | wanted      |
| `R-42` | Windows x86_64 qualification; deferred Linux aarch64 and macOS current-build re-test | [G7](goals.md#goals)                       | Windows next; others deferred       | unranked                                | Windows peer; real-machine access for deferred targets                           | Windows peer receives a cross-compiled binary, no build environment; macOS tested on an earlier build only                                                                                                                   | unseeded           | wanted      |

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

- Disposition: absorbed. One blocking thread per lane with no async runtime on TX and RX is design [Thread-Per-Lane Blocking I/O](design.md#thread-per-lane-blocking-io), with [Rust Crate Set](design.md#rust-crate-set) removing the runtime entirely. That design decision does not pin lane threads to cores.

<a id="r-34-linux-gro-receive-and-lane-steering"></a>

### R-34 Linux RX Adapter Receive Tuning

- User need: the Linux RX host's network adapter passes every lane's datagrams to RX at line rate, without the rare receive-ring misses of the [known issues](known-issues.md#register).
- `R-34.S-01`: where the adapter supports n-tuple filters and more than one receive queue, `ethtool -N` n-tuple rules steer destination port base + k to queue k, and IRQ affinity places the queues on separate cores.
- `R-34.S-03`: `ethtool -G rx` sets the RX host adapter's receive ring to its maximum size.
- Evidence: on the measured 10GbE link the RX adapter runs one receive queue with a 256-entry ring, and the rare residual loss at about 9.4 Gbit/s is receive-ring misses in that adapter, which `--ipd-ns` pacing and larger RX socket buffers do not remove ([design](design.md#measured-link-behavior), [known issues](known-issues.md#register)).
- Assumptions: SO_REUSEPORT and BPF steering are not needed because each lane has its own port [speculative]; without n-tuple support RSS hashes the 4-tuple and lanes may share a queue, which `ethtool -S` shows [speculative].
- Prospective acceptance `R-34.AC-01`: during a measured run (`R-39.S-06`), the adapter's receive-missed counter and Udp RcvbufErrors in `/proc/net/snmp` stay at zero, and with steering in place per-queue packets in `ethtool -S` show lane k on queue k.
- Applicability: host and adapter configuration on the RX host, applied by an administrator; RX's receive path is unchanged. Steering needs NIC n-tuple and multi-queue support, which is common on 10G+. Kernel busy polling (`SO_BUSY_POLL`, `napi_defer_hard_irqs`) is outside the design [Wait Policy](design.md#wait-policy).
- Open question: the measured RX adapter's maximum receive ring size, and whether it supports n-tuple filters and more than one receive queue.

### R-35 Absorbed: Slab Pool And Reassembly Ring

- Disposition: absorbed. The fixed pools are design [No Heap Allocation In The Critical Path](design.md#no-heap-allocation-in-the-critical-path), and the sequence-indexed ring is the wheel of design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo).

### R-36 Jumbo MTU End To End

- User need: fewer datagrams per second for the same rate.
- `R-36.S-01`: MTU 9000 on both ends and on every hop between them.
- `R-36.S-02`: the datagram size at most the path MTU less 48 bytes (the 40-byte IPv6 and 8-byte UDP headers), which is 8952 at MTU 9000 and is the default the [specification](spec.md#semantics) derives from a 9000-byte adapter MTU; a larger datagram fragments and loses data ([design](design.md#measured-link-behavior)).
- Rationale: 10 Gbit/s is about 830k datagrams/s at 1472 B payload and about 140k/s at 9000 B, about 6× fewer [arithmetic; gain speculative]; spead2 gives jumbo-frame guidance ([prior art](research/prior-art.md), item 1).
- Prospective acceptance `R-36.AC-01`: the wire baseline (`R-39.S-03`) runs with and without jumbo frames on the target path.
- Applicability: needs a switch and NIC path that carries jumbo frames; `scripts/admin/sm-mtu.sh` prints the commands an administrator runs.
- Readiness: deferred until the Windows peer exists (`R-42`); the available Linux hosts use a 1500-byte MTU.

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
- Assumption: the counter names in [max bandwidth](research/max-bandwidth.md) were not re-checked [speculative].
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

## Dependencies

`R-08` records the crate set without an async runtime, and `R-31` is absorbed into the thread-per-lane design. `R-18` uses the seed of design [TX-PAT Seed Option](design.md#tx-pat-seed-option) and validates the output that design [Output Under Loss](design.md#output-under-loss) describes. `R-22` is the validator that `R-18` describes. `R-34` is judged by the Linux counters of `R-39`, and its steering part needs an adapter with n-tuple filters and more than one receive queue. `R-41` waits on a stable MVP release and relates to `R-39`. `R-42` puts Windows x86_64 qualification next, pending its peer, while Linux aarch64 qualification and macOS re-testing of the current build remain deferred. `R-36` waits on the Windows peer. `R-38` waits on `R-39`, and `R-34` and `R-36` are tuned from it.

## Readiness

A prospect is ready for a Change when its behavior is in the [specification](spec.md) and its mechanism is in the [design](design.md). Readiness does not activate work.

## Boundaries

Outcomes are in the [goals](goals.md), current behavior in the [specification](spec.md), current mechanism and decisions in the [design](design.md), forbidden implementation patterns in the [constraints](constraints.md), finite residuals in [known issues](known-issues.md), and background research in [research](research/). A prospect whose decision enters its owner retires here with a successor reference, and this roadmap records its disposition. This roadmap records no execution or release history. Retransmission is outside the horizon at this stage.
