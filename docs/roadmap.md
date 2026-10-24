---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-roadmap]; otherwise, do not modify."
---

# socket-mux Roadmap

## Purpose

Hold the prospective evolution of socket-mux, both TX and RX, for anyone deciding what to build next: every prospect, with its prospective requirements and acceptance criteria. It also holds each outcome wanted later, with its sequencing. Nothing in this roadmap is a decision on behavior or mechanism.

## Orientation

Each `SM-D` and `SM-R` prospect is a prospective change; the disposition `superseded` or `absorbed` marks one that the [design](design.md) replaced or took in. Each `SM-W` prospect is a wanted outcome, and its mechanics are unspecified. The `SM-D` prospects keep the identities that the [specification](spec.md) cites. The `SM-R` prospects are throughput prospects from the research in [max bandwidth](research/max-bandwidth.md) and [prior art](research/prior-art.md); that research carries no authority, and its tags [proven], [vendor-claimed] and [speculative] stay at each claim. `REQ` and `AC` identifiers name a prospect's prospective requirements and acceptance criteria. `M` identifiers name cases of the back-to-back verification loop ([Back-To-Back Verification Loop](design.md#back-to-back-verification-loop)). Goals owners are the outcomes in [goals](goals.md#goals). `unranked` means no priority is set, and `unseeded` means no Change has been seeded.

## Horizon

| id       | outcome                                            | goals owner                                | horizon                             | priority                               | dependencies                                           | readiness                                                                                                                                                                           | prospective Change | disposition |
| -------- | -------------------------------------------------- | ------------------------------------------ | ----------------------------------- | -------------------------------------- | ------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ | ----------- |
| `SM-D8`  | Rust crate set for TX and RX                       | [G7](goals.md#goals)                       | open                                | unranked                               | `SM-R1`                                                | crate analysis only                                                                                                                                                                 | unseeded           | proposed    |
| `SM-D9`  | Round-robin slice assignment                       | [G1](goals.md#goals)                       | open                                | unranked                               | none                                                   | verification case M07 expects it                                                                                                                                                    | unseeded           | proposed    |
| `SM-D10` | Datagram header layout                             | [G3](goals.md#goals)                       | open                                | unranked                               | none                                                   | specification carries it                                                                                                                                                            | unseeded           | proposed    |
| `SM-D12` | Default ports                                      | [G5](goals.md#goals)                       | open                                | unranked                               | none                                                   | specification carries it                                                                                                                                                            | unseeded           | proposed    |
| `SM-D13` | RX skips and logs loss gaps                        | [G3](goals.md#goals); [G4](goals.md#goals) | open                                | unranked                               | `SM-D10`                                               | specification carries it                                                                                                                                                            | unseeded           | proposed    |
| `SM-D14` | RX delivers the stream over TCP                    | [G1](goals.md#goals); [G6](goals.md#goals) | open                                | unranked                               | none                                                   | specification carries it                                                                                                                                                            | unseeded           | proposed    |
| `SM-D15` | `socket-mux` command line                          | [G2](goals.md#goals); [G5](goals.md#goals) | open                                | unranked                               | `SM-D12`                                               | specification carries it                                                                                                                                                            | unseeded           | proposed    |
| `SM-D16` | `tx` input operand                                 | [G1](goals.md#goals)                       | open                                | unranked                               | `SM-D15`                                               | specification carries it                                                                                                                                                            | unseeded           | proposed    |
| `SM-D17` | `tx-pat` seed option                               | [G6](goals.md#goals)                       | open                                | unranked                               | `SM-D15`                                               | specification carries it                                                                                                                                                            | unseeded           | proposed    |
| `SM-D18` | Self-locating Adler pattern blocks and validator   | [G3](goals.md#goals); [G6](goals.md#goals) | open                                | unranked                               | `SM-D17`                                               | the specification marks the pattern format and the validator as gaps                                                                                                                | unseeded           | proposed    |
| `SM-D21` | Spelling of the inter-packet delay timeout         | [G3](goals.md#goals)                       | open                                | unranked                               | `SM-D15`                                               | design [Inter-Packet Delay Timeout](design.md#inter-packet-delay-timeout) states the timeout and its deadline; this prospect fills in its spelling                                  | unseeded           | proposed    |
| `SM-D22` | `check-pat` pattern checker subcommand             | [G6](goals.md#goals)                       | open                                | unranked                               | `SM-D18`                                               | the verification loop uses its own checker                                                                                                                                          | unseeded           | proposed    |
| `SM-D23` | Provisional cap on held datagrams                  | [G1](goals.md#goals); [G3](goals.md#goals) | closed                              | unranked                               | none                                                   | replaced by the arena of design [RX Wheel, Arena And FIFO](design.md#rx-wheel-arena-and-fifo)                                                                                       | unseeded           | superseded  |
| `SM-R1`  | Pinned OS thread per lane; tokio only for control  | [G1](goals.md#goals)                       | open                                | Win64 send rank 1 in research          | `SM-D8`; `SM-R9`                                       | research only; unmeasured                                                                                                                                                           | unseeded           | proposed    |
| `SM-R2`  | quinn-udp for GSO, GRO and USO                     | [G1](goals.md#goals)                       | open                                | unranked                               | `SM-D8`; `SM-R9`                                       | research only; unmeasured                                                                                                                                                           | unseeded           | proposed    |
| `SM-R3`  | Win64 USO in 64 KB batches                         | [G1](goals.md#goals)                       | open                                | Win64 send rank 2 in research          | `SM-R2`; `SM-D10`; NIC and driver USO support          | research only; unmeasured                                                                                                                                                           | unseeded           | proposed    |
| `SM-R4`  | Linux GRO receive with steered, pinned lanes       | [G1](goals.md#goals)                       | open                                | Linux receive ranks 1 to 3 in research | `SM-R2`; `SM-R7`; NIC n-tuple support                  | research only; unmeasured                                                                                                                                                           | unseeded           | proposed    |
| `SM-R5`  | Slab pool and sequence-indexed reassembly ring     | [G1](goals.md#goals); [G3](goals.md#goals) | closed                              | unranked                               | none                                                   | absorbed into design [RX Wheel, Arena And FIFO](design.md#rx-wheel-arena-and-fifo) and [No Heap Allocation In The Critical Path](design.md#no-heap-allocation-in-the-critical-path) | unseeded           | absorbed    |
| `SM-R6`  | Jumbo MTU end to end                               | [G1](goals.md#goals)                       | open                                | Win64 send rank 3 in research          | `--datagram-size`; a jumbo-capable switch and NIC path | research only; unmeasured                                                                                                                                                           | unseeded           | proposed    |
| `SM-R7`  | Larger default `--udp-buffer`                      | [G1](goals.md#goals)                       | open                                | Linux receive rank 3 in research       | `SM-D15`                                               | harness failures seen at operating-system default buffers; no value chosen                                                                                                          | unseeded           | proposed    |
| `SM-R8`  | RIO, io_uring zcrx and AF_XDP                      | [G1](goals.md#goals)                       | only if the socket path falls short | last in research                       | `SM-R9`                                                | research only; NIC and driver dependent                                                                                                                                             | unseeded           | deferred    |
| `SM-R9`  | Per-layer throughput measurement                   | [G1](goals.md#goals)                       | open                                | unranked                               | a TX and RX binary                                     | research only                                                                                                                                                                       | unseeded           | proposed    |
| `SM-W1`  | /goal optimization loop for latency and throughput | [G1](goals.md#goals)                       | after a stable MVP release          | unranked                               | a stable MVP release; relates to `SM-R9`               | wanted; mechanics unspecified; waits on a stable MVP release                                                                                                                        | unseeded           | wanted      |

## Prospect definitions

### SM-D8 Rust Crate Set

| Field             | Value                                                                                                                                                                                                                                                                                                                                                      |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | If the language is Rust: tokio with the current_thread runtime and minimal features; socket2 for socket buffers and options; bytes; windows-sys to disable SIO_UDP_CONNRESET on Windows; optionally quinn-udp for batched I/O later; lexopt or pico-args for arguments; anyhow for errors; log with env_logger for logging.                                |
| Rationale         | From a crate analysis of the TX and RX mechanism.                                                                                                                                                                                                                                                                                                          |
| Alternatives      | lexopt and pico-args are alternatives to each other; the choice is open.                                                                                                                                                                                                                                                                                   |
| Rejection reason  | None recorded.                                                                                                                                                                                                                                                                                                                                             |
| Affected surfaces | All dependencies                                                                                                                                                                                                                                                                                                                                           |

- User need: a dependency set that builds TX and RX for Linux, macOS and Windows ([Cross-Platform Dependencies](design.md#cross-platform-dependencies)).
- `SM-D8-REQ1`: the crate set in the Proposal row above.
- Prospective acceptance `SM-D8-AC1`: every listed crate is compatible with Linux, Mac and Windows ([Cross-Platform Dependencies](design.md#cross-platform-dependencies)); the build and test targets are open.
- Applicability: only if the language is Rust.
- Open questions: the language and toolchain; lexopt or pico-args; `SM-R1` proposes a different data-path runtime than the tokio current_thread pick.

### SM-D9 Round-Robin Slice Assignment

| Field             | Value                                                                                                          |
| ----------------- | -------------------------------------------------------------------------------------------------------------- |
| Proposal          | TX assigns slices to lanes by plain round-robin, one datagram per lane in turn.                                |
| Rationale         | None recorded.                                                                                                 |
| Alternatives      | None recorded.                                                                                                 |
| Rejection reason  | None recorded.                                                                                                 |
| Affected surfaces | TX path; SM-P04                                                                                                |

- User need: TX spreads one stream over the N lanes (G1).
- `SM-D9-REQ1`: the assignment in the Proposal row above.
- Prospective acceptance `SM-D9-AC1`: with N > 1, a burst of C datagrams dropped on one lane is reported as C gaps of one slice each.
- Open question: the policy that assigns slices to lanes.

### SM-D10 Completed: Datagram Header Layout

- Disposition: completed. The [specification](spec.md#datagram-header) carries it, and the decision is design [Datagram Header Layout](design.md#datagram-header-layout).

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
| Field             | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Proposal          | The per-datagram header ([Header Per UDP Datagram](design.md#header-per-udp-datagram)) is 14 + S/8 bytes in network byte order, 18 for the normal 32-bit sequence width of design [Sequence Width Is A Build Constant](design.md#sequence-width-is-a-build-constant): magic, version, flags, payload length, sequence and stream offset, as laid out in the [specification](spec.md#datagram-header).                                                                                                                                                                                                  |
| Rationale         | magic: RX discards stray datagrams that reach its ports. version: the layout can change without misreading old datagrams. flags: the end-of-stream bit tells RX that the TCP stream closed, so RX can close its output. payload length: RX knows how many slice bytes follow, independent of datagram padding. sequence: an S-bit global counter orders slices across all N lanes and counts lost slices; it rolls over (design [Sequence Width Is A Build Constant](design.md#sequence-width-is-a-build-constant)). stream offset: gives the exact byte length of a loss gap when payload sizes vary. |
| Alternatives      | A sequence number without a stream offset.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Rejection reason  | Without a stream offset the byte length of a loss is known only if every payload has the same size.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Affected surfaces | SM-P05; TX path; RX path                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
- Prospective acceptance `SM-D10-AC1`: the verification loop's independent decoder reads every datagram TX sends with this layout (M05 to M08).
- Open question: the header layout.
- Open question: what RX emits where it detects loss.
| Proposal          | The binary is named `socket-mux` and takes the grammar `socket-mux [GLOBAL OPTIONS] tx [TX OPTIONS]` or `socket-mux [GLOBAL OPTIONS] rx [RX OPTIONS]`, with the subcommands spelled `tx` and `rx`, plus `socket-mux [GLOBAL OPTIONS] tx-pat [TX-PAT OPTIONS]` with the spelling `tx-pat`, and command options after the subcommand. Global options are accepted before and after the subcommand. The option set, short forms, defaults, constraints, precedence, invalid combinations and exit statuses in the [specification](spec.md#command-line), independent of any argument-parsing library. The `tx` `INPUT` operand is `SM-D16` and the `tx-pat` `--seed` option is `SM-D17`.                                                                               |
| Rationale         | Global options carry the settings both sides must agree on (N, base port, datagram size) or share (logging, UDP buffer size, arena size); the default `--datagram-size` of 1452 bytes is a 1500-byte MTU less the 40-byte IPv6 and 8-byte UDP headers, so one datagram fits one frame on IPv6 and IPv4 without fragmentation; the default `--arena-size` of 64 MiB holds W = 46217 datagrams of 1452 bytes, about 54 ms of arrivals at 10 Gbit/s and 0.5 s at 1 Gbit/s, a small part of a host's memory; command options carry settings only one side uses. Command options stay after the subcommand, and early usage errors stop a misconfigured side before it opens a socket.                                                                                   |

### SM-D18 Self-Locating Adler Pattern Blocks

| Field             | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | The pattern is a sequence of 4096-byte blocks. Block i holds i as an 8-byte big-endian index, then 4084 bytes from a pseudorandom generator keyed by the seed and i, then the Adler-32 checksum of the first 4092 bytes. A validator reading RX's output checks each block's Adler-32, and with the seed also regenerates its body. After a loss gap it slides one byte at a time until a 4096-byte window has a matching Adler-32 and, with the seed, a body equal to the regeneration for its index; it then reports the skipped byte count and continues. The validator is a separate checker that reads RX's TCP output; RX does not change.                                        |
| Rationale         | The index and checksum in every block let validation find its place again after any loss, adversarial loss included, without trusting RX's offsets. The Adler-32 alone checks integrity when the seed was random and not recorded.                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Alternatives      | An unframed generator stream located by the header stream offset; RX validating the pattern itself.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Rejection reason  | RX's output skips loss gaps (`SM-D13`), so the validator does not see stream offsets; validation inside RX would check RX with its own code.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| Affected surfaces | SM-P13; `tx-pat`; verification loop                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

- User need: the review cycle validates RX against a known pattern and recovers after adversarial UDP loss (G3, G6).
- `SM-D18-REQ1`: the block format and validator in the Proposal row above.
- Prospective acceptance `SM-D18-AC1`: with a fixed seed and no loss, every block validates; with any set of dropped datagrams, the validator recovers within one block after each gap and every delivered block that is whole validates.
- Assumption: 4096 bytes is a starting block size, not measured against the datagram size.
- Open questions: the pattern format; the validator; whether and how `tx-pat` ends.

### SM-D21 Inter-Packet Delay Timeout Spelling
| Field             | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | The inter-packet delay timeout of design [Inter-Packet Delay Timeout](design.md#inter-packet-delay-timeout) is spelled `--timeout SECONDS`, a decimal number above 0 with default 10, accepted before or after every subcommand like the other global options; TX and `tx-pat` accept it and do not use it. Under rule A RX logs the expired gap and resumes delivery; under rule B it logs each gap and one info line naming the timeout and the loss totals, closes its output and exits 0. |
| Rationale         | A global option fits a setting both sides accept, and one info line at the end of the stream reports the loss totals once.                                                                                                                                                                                                                                                                                                                                                                    |
| Alternatives      | None recorded.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Rejection reason  | None recorded.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Affected surfaces | Global options; RX output; SM-P09                                                                                                                                                                                                                                                                                                                                                                                                                                                             |

- Disposition: completed. The [specification](spec.md#global-options) carries it, and the decision is design [Inter-Packet Delay Timeout Spelling](design.md#inter-packet-delay-timeout-spelling).
- `SM-D21-REQ1`: the spelling and the end-of-stream log line in the Proposal row above.
- Open questions: none.

### SM-D22 check-pat Checker Subcommand

| Field             | Value                                                                                                                                                                                                                                                                                                                                       |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | The pattern validator of SM-D18 is a `check-pat [--seed U64]` subcommand of `socket-mux`. It reads RX's output on standard input, prints one line with the counts of valid and corrupt blocks, resynchronizations, lost bytes and trailing bytes, and exits with status 0 when no block is corrupt and 1 otherwise.                         |
| Rationale         | Operators who run `tx-pat` get a validator in the same binary, on every platform.                                                                                                                                                                                                                                                           |
| Alternatives      | A separate checker program; only the verification loop's own checker.                                                                                                                                                                                                                                                                       |
| Rejection reason  | None recorded.                                                                                                                                                                                                                                                                                                                              |
| Affected surfaces | SM-P13; CLI                                                                                                                                                                                                                                                                                                                                 |

- User need: validate RX's output against the pattern without extra tools (G6).
- `SM-D22-REQ1`: the subcommand in the Proposal row above.
- Prospective acceptance `SM-D22-AC1`: on clean, lossy, truncated and corrupted pattern streams, its counts equal those of an independent checker.
- Open question: which program validates the pattern.

### SM-D23 Superseded: Provisional Cap On Held Datagrams

- Disposition: superseded. The arena of design [RX Wheel, Arena And FIFO](design.md#rx-wheel-arena-and-fifo) bounds the datagrams RX holds, under the loss limits in the [specification](spec.md#semantics).

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
- Applicability: makes the optional "quinn-udp for batched I/O later" in `SM-D8` the batched I/O path.
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

- Disposition: absorbed. The fixed pools are design [No Heap Allocation In The Critical Path](design.md#no-heap-allocation-in-the-critical-path), and the sequence-indexed ring is the wheel of design [RX Wheel, Arena And FIFO](design.md#rx-wheel-arena-and-fifo).

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
- Open question: the measurement hosts and NICs, which the build and test targets do not yet name.

### SM-W1 Wanted: Iterative Optimization Loop For Latency And Throughput

- User need: an iterative optimization loop for latency and throughput, wanted after a stable MVP release.
- Outcome: an iterative optimization loop that improves socket-mux latency and throughput.
- Sequencing: the loop follows a stable MVP release.
- Relation: the per-layer measurement plan `SM-R9` names the rates and counters per layer that such a loop could measure.
- Unspecified: the loop's mechanics, its latency and throughput metrics and targets, and what makes an MVP release stable. No requirement or acceptance criterion is drafted.

## Dependencies

`SM-D8` and `SM-R1` propose different data-path runtimes; the language and toolchain choose between them. `SM-R2` turns the optional quinn-udp of `SM-D8` into the batched path that `SM-R3` and `SM-R4` use. `SM-R3` puts one `SM-D10` header in each USO segment. `SM-R4` needs equal-size segments from TX and the receive buffers of `SM-R7`. `SM-R6` bears on the default `--datagram-size`. `SM-R7` changes a default that `SM-D15` proposes. `SM-D13` uses the sequence and stream offset of `SM-D10`. `SM-D15` carries the defaults of `SM-D12`, and its grammar carries the operand of `SM-D16` and the option of `SM-D17`. `SM-D18` uses the seed of `SM-D17` and validates the output that `SM-D13` produces under loss. `SM-D21` fills in the spelling of the timeout that design [Inter-Packet Delay Timeout](design.md#inter-packet-delay-timeout) states. `SM-D22` is the validator that `SM-D18` describes. `SM-W1` waits on a stable MVP release and relates to `SM-R9`. `SM-R8` waits on `SM-R9`, and `SM-R1` to `SM-R7` are tuned from it.

## Readiness

A prospect is ready for a Change when its behavior is in the [specification](spec.md) and its mechanism is in the [design](design.md). Readiness does not activate work.

## Boundaries

Outcomes are in the [goals](goals.md), current behavior in the [specification](spec.md), current mechanism and decisions in the [design](design.md), and background research in [research](research/). A prospect whose decision enters its owner retires here with a successor reference, and this roadmap records its disposition. This roadmap records no execution or release history. Retransmission is outside the horizon at this stage.
