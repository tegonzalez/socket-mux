---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-roadmap]; otherwise, do not modify."
---

# socket-mux Roadmap

## Purpose

Hold the prospective evolution of socket-mux, both TX and RX, for anyone deciding what to approve or build next: every PROPOSAL drafted for approval, with its prospective requirements and acceptance criteria. It also holds each outcome wanted later, with its sequencing. Nothing in this roadmap is a decision on behavior or mechanism.

## Orientation

Each `SM-D` and `SM-R` prospect is a PROPOSAL and is not approved. Each `SM-W` prospect is a wanted outcome; it is not a PROPOSAL, and its mechanics are unspecified. The `SM-D` prospects keep the identities that the [specification](spec.md) cites. The `SM-R` prospects are throughput proposals from Scout's research in [max bandwidth](research/max-bandwidth.md) and [prior art](research/prior-art.md); that research carries no authority, and its tags [proven], [vendor-claimed] and [speculative] stay at each claim. `REQ` and `AC` identifiers name a prospect's prospective requirements and acceptance criteria. `M` identifiers name cases of the back-to-back verification loop (SM-D6). Goals owners are the outcomes in [goals](goals.md#goals). `unranked` means no priority is set, and `unseeded` means no Change has been seeded.

## Horizon

| id       | outcome                                            | goals owner                                | horizon                                | priority                               | dependencies                                  | readiness                                                                  | prospective Change | disposition |
| -------- | -------------------------------------------------- | ------------------------------------------ | -------------------------------------- | -------------------------------------- | --------------------------------------------- | -------------------------------------------------------------------------- | ------------------ | ----------- |
| `SM-D8`  | Rust crate set for TX and RX                       | [G7](goals.md#goals)                       | on Tomas's decision of OD-1            | unranked                               | OD-1; `SM-R1`                                 | not reviewed by Tomas                                                      | unseeded           | proposed    |
| `SM-D9`  | Round-robin slice assignment                       | [G1](goals.md#goals)                       | on Tomas's decision of OD-4            | unranked                               | OD-4                                          | verification case M07 expects it                                           | unseeded           | proposed    |
| `SM-D10` | 22-byte datagram header layout                     | [G3](goals.md#goals)                       | on Tomas's decision of OD-12           | unranked                               | OD-12; OD-3                                   | specification carries it as PROPOSAL                                       | unseeded           | proposed    |
| `SM-D12` | Default N and default ports                        | [G5](goals.md#goals)                       | on Tomas's decision of OD-14           | unranked                               | OD-14                                         | specification carries it as PROPOSAL                                       | unseeded           | proposed    |
| `SM-D13` | RX skips and logs loss gaps                        | [G3](goals.md#goals); [G4](goals.md#goals) | on Tomas's decision of OD-15           | unranked                               | OD-15; OD-5; `SM-D10`                         | specification carries it as PROPOSAL                                       | unseeded           | proposed    |
| `SM-D14` | RX delivers the stream over TCP                    | [G1](goals.md#goals); [G6](goals.md#goals) | on Tomas's decision of OD-16           | unranked                               | OD-16                                         | specification carries it as PROPOSAL                                       | unseeded           | proposed    |
| `SM-D15` | `socket-mux` command line                          | [G2](goals.md#goals); [G5](goals.md#goals) | on Tomas's decision of OD-9            | unranked                               | OD-9; `SM-D12`                                | specification carries it as PROPOSAL                                       | unseeded           | proposed    |
| `SM-D16` | `tx` input operand                                 | [G1](goals.md#goals)                       | on Tomas's decision of OD-17           | unranked                               | OD-17; `SM-D15`                               | specification carries it as PROPOSAL                                       | unseeded           | proposed    |
| `SM-D17` | `tx-pat` seed option                               | [G6](goals.md#goals)                       | on Tomas's decision of OD-18           | unranked                               | OD-18; `SM-D15`                               | specification carries it as PROPOSAL                                       | unseeded           | proposed    |
| `SM-D18` | Self-locating Adler pattern blocks and validator   | [G3](goals.md#goals); [G6](goals.md#goals) | on Tomas's decision of OD-19 and OD-20 | unranked                               | OD-19; OD-20; OD-21; `SM-D17`                 | specification carries it as PROPOSAL                                       | unseeded           | proposed    |
| `SM-D21` | Timer rules and spelling of the global timeout     | [G3](goals.md#goals)                       | on Tomas's review                      | unranked                               | `SM-D15`; OD-5                                | design SM-D19 decided the timeout; this PROPOSAL fills in its rules        | unseeded           | proposed    |
| `SM-D22` | `check-pat` pattern checker subcommand             | [G6](goals.md#goals)                       | on Tomas's decision of OD-20           | unranked                               | OD-20; `SM-D18`                               | Tomas moved it to the roadmap; the verification loop uses its own checker  | unseeded           | proposed    |
| `SM-R1`  | Pinned OS thread per leg; tokio only for control   | [G1](goals.md#goals)                       | on Tomas's decision                    | Win64 send rank 1 in research          | `SM-D8`; `SM-R9`                              | research only; unmeasured                                                  | unseeded           | proposed    |
| `SM-R2`  | quinn-udp for GSO, GRO and USO                     | [G1](goals.md#goals)                       | on Tomas's decision                    | unranked                               | `SM-D8`; `SM-R9`                              | research only; unmeasured                                                  | unseeded           | proposed    |
| `SM-R3`  | Win64 USO in 64 KB batches                         | [G1](goals.md#goals)                       | on Tomas's decision                    | Win64 send rank 2 in research          | `SM-R2`; `SM-D10`; NIC and driver USO support | research only; unmeasured                                                  | unseeded           | proposed    |
| `SM-R4`  | Linux GRO receive with steered, pinned legs        | [G1](goals.md#goals)                       | on Tomas's decision                    | Linux receive ranks 1 to 3 in research | `SM-R2`; `SM-R7`; NIC n-tuple support         | research only; unmeasured                                                  | unseeded           | proposed    |
| `SM-R5`  | Slab pool and sequence-indexed reassembly ring     | [G1](goals.md#goals); [G3](goals.md#goals) | on Tomas's decision                    | unranked                               | `SM-D10`; OD-5; OD-7                          | research only; unmeasured                                                  | unseeded           | proposed    |
| `SM-R6`  | Jumbo MTU end to end                               | [G1](goals.md#goals)                       | on Tomas's decision                    | Win64 send rank 3 in research          | OD-3; a jumbo-capable switch and NIC path     | research only; unmeasured                                                  | unseeded           | proposed    |
| `SM-R7`  | Larger default `--udp-buffer`                      | [G1](goals.md#goals)                       | on Tomas's decision                    | Linux receive rank 3 in research       | `SM-D15`                                      | harness failures seen at operating-system default buffers; no value chosen | unseeded           | proposed    |
| `SM-R8`  | RIO, io_uring zcrx and AF_XDP                      | [G1](goals.md#goals)                       | only if the socket path falls short    | last in research                       | `SM-R9`                                       | research only; NIC and driver dependent                                    | unseeded           | deferred    |
| `SM-R9`  | Per-layer throughput measurement                   | [G1](goals.md#goals)                       | on Tomas's decision                    | unranked                               | a TX and RX binary                            | research only                                                              | unseeded           | proposed    |
| `SM-W1`  | /goal optimization loop for latency and throughput | [G1](goals.md#goals)                       | after a stable MVP release             | unranked                               | a stable MVP release; relates to `SM-R9`      | wanted by Tomas; mechanics unspecified; waits on a stable MVP release      | unseeded           | wanted      |

## Prospect definitions

### SM-D8 PROPOSAL: Rust Crate Set

| Field             | Value                                                                                                                                                                                                                                                                                                                                                      |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | PROPOSAL, not approved. If the language is Rust (OD-1): tokio with the current_thread runtime and minimal features; socket2 for socket buffers and options; bytes; windows-sys to disable SIO_UDP_CONNRESET on Windows; optionally quinn-udp for batched I/O later; lexopt or pico-args for arguments; anyhow for errors; log with env_logger for logging. |
| Rationale         | From a crate analysis of the decided TX and RX mechanism; not reviewed.                                                                                                                                                                                                                                                                                    |
| Alternatives      | lexopt and pico-args are alternatives to each other; the choice is open.                                                                                                                                                                                                                                                                                   |
| Rejection reason  | None recorded.                                                                                                                                                                                                                                                                                                                                             |
| Affected surfaces | All dependencies                                                                                                                                                                                                                                                                                                                                           |

- User need: a dependency set that builds TX and RX for Linux, macOS and Windows (SM-D7).
- `SM-D8-REQ1`: the crate set in the Proposal row above.
- Prospective acceptance `SM-D8-AC1`: every listed crate is compatible with Linux, Mac and Windows (SM-D7); the build and test targets are open (OD-11).
- Applicability: only if the language is Rust (OD-1).
- Open questions: OD-1; lexopt or pico-args; `SM-R1` proposes a different data-path runtime than the tokio current_thread pick.

### SM-D9 PROPOSAL: Round-Robin Slice Assignment

| Field             | Value                                                                                                          |
| ----------------- | -------------------------------------------------------------------------------------------------------------- |
| Proposal          | PROPOSAL, not approved. TX assigns slices to lanes by plain round-robin, one datagram per lane in turn (OD-4). |
| Rationale         | None recorded.                                                                                                 |
| Alternatives      | None recorded.                                                                                                 |
| Rejection reason  | None recorded.                                                                                                 |
| Affected surfaces | TX path; SM-P04                                                                                                |

- User need: TX spreads one stream over the N lanes (G1).
- `SM-D9-REQ1`: the assignment in the Proposal row above.
- Prospective acceptance `SM-D9-AC1`: with N > 1, a burst of C datagrams dropped on one lane is reported as C gaps of one slice each (M07).
- Open question: OD-4.

### SM-D10 PROPOSAL: Datagram Header Layout

| Field             | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Proposal          | PROPOSAL, not approved. The per-datagram header (SM-D2) is 22 bytes in network byte order: magic, version, flags, payload length, sequence and stream offset, as laid out in the [specification](spec.md#datagram-header) (OD-12).                                                                                                                                                                                                                                                                                                                                                         |
| Rationale         | magic: RX discards stray datagrams that reach its ports. version: the layout can change without misreading old datagrams. flags: the end-of-stream bit tells RX that the TCP stream closed, so RX can close its output. payload length: RX knows how many slice bytes follow, independent of datagram padding. sequence: a 64-bit global counter orders slices across all N lanes and counts lost slices, and does not wrap in practice (at 10 million datagrams per second it lasts over 58,000 years). stream offset: gives the exact byte length of a loss gap when payload sizes vary. |
| Alternatives      | A 32-bit sequence number; a sequence number without a stream offset.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Rejection reason  | A 32-bit counter wraps after about 7 minutes at 10 million datagrams per second; without a stream offset the byte length of a loss is known only if every payload has the same size.                                                                                                                                                                                                                                                                                                                                                                                                       |
| Affected surfaces | SM-P05; TX path; RX path                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |

- User need: RX sequences slices and determines packet loss and the length of that loss from every datagram (G3, SM-D2).
- `SM-D10-REQ1`: the layout in the Proposal row above, with the field table in the [specification](spec.md#datagram-header).
- Prospective acceptance `SM-D10-AC1`: the verification loop's independent decoder reads every datagram TX sends with this 22-byte layout (M05 to M08).
- Open questions: OD-12; the payload size (OD-3).

### SM-D12 PROPOSAL: Default Count And Ports

| Field             | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | PROPOSAL, not approved. Default N is 4. The default UDP base port is 31000, so the default lanes use 31000 to 31003. TX accepts TCP ingress on 127.0.0.1:31100 by default, and RX delivers to 127.0.0.1:31200 by default (OD-14).                                                                                                                                                                                                                                                   |
| Rationale         | The default port is meant to be straightforward and above the typical range. 31000 is round and memorable, lies above the well-known ports (0 to 1023) and below every ephemeral range (Linux 32768 to 60999; IANA, Windows and macOS 49152 to 65535), and IANA lists 31000 to 31015 as unassigned, so N up to 16 stays inside that block. 31100 and 31200 follow the same logic in the unassigned block 31030 to 31336. N = 4 is the smallest power of two above the minimum of 3. |
| Alternatives      | A base port in the IANA dynamic range (49152 to 65535).                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Rejection reason  | It collides with ephemeral ports the operating system assigns to outgoing connections.                                                                                                                                                                                                                                                                                                                                                                                              |
| Affected surfaces | SM-P03, SM-P04, SM-P07, SM-P08; CLI defaults                                                                                                                                                                                                                                                                                                                                                                                                                                        |

- User need: N is configurable with a minimum of three and a configurable default (G5), on ports outside the ephemeral ranges.
- `SM-D12-REQ1`: the defaults in the Proposal row above.
- Prospective acceptance `SM-D12-AC1`: with no `-n`, RX binds the default number of ports, 31000 to 31003 (M02).
- Open question: OD-14.

### SM-D13 PROPOSAL: Output Under Loss

| Field             | Value                                                                                                                                                                                                                                           |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | PROPOSAL, not approved. Where RX detects loss, it skips the lost bytes, keeps the order of the bytes it delivers, and logs the gap with its lost slice count (from sequence) and byte length (from stream offset), plus running totals (OD-15). |
| Rationale         | The channel is lossy with no retransmission (SM-D3), so RX keeps the stream flowing and makes every loss visible.                                                                                                                               |
| Alternatives      | Fill the gap with zero bytes of the lost length; end the stream at the first loss.                                                                                                                                                              |
| Rejection reason  | Zero fill injects bytes the sender never sent; ending the stream turns every loss into a failure on a channel that is lossy by decision.                                                                                                        |
| Affected surfaces | SM-P07, SM-P09; RX path                                                                                                                                                                                                                         |

- User need: the stream keeps flowing on a lossy channel and every loss is visible (G3, G4).
- `SM-D13-REQ1`: the behavior in the Proposal row above.
- Prospective acceptance `SM-D13-AC1`: RX's output equals the original minus exactly the dropped byte ranges, and RX's logged totals equal the dropped slice count and byte length (M05 to M08).
- Open questions: OD-15; when RX declares a slice lost rather than late (OD-5).

### SM-D14 PROPOSAL: RX Delivery Over TCP

| Field             | Value                                                                                                                                                             |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | PROPOSAL, not approved. RX connects out over TCP to a configurable peer, `--connect`, default 127.0.0.1:31200, and writes the reconstituted stream to it (OD-16). |
| Rationale         | It mirrors TX's TCP ingress, so the pair forms a TCP-to-TCP link and a back-to-back verification loop needs only a TCP client and a TCP sink.                     |
| Alternatives      | RX listens for a TCP client; RX writes to standard output.                                                                                                        |
| Rejection reason  | Listening adds a second accept step before data can flow; standard output does not give a TCP stream on the far side.                                             |
| Affected surfaces | SM-P07; `--connect`                                                                                                                                               |

- User need: the reconstituted stream reaches a TCP peer, so TX and RX form a verification loop (G1, G6).
- `SM-D14-REQ1`: the delivery in the Proposal row above.
- Prospective acceptance `SM-D14-AC1`: in the lossless loop, the bytes read at RX's `--connect` peer equal the bytes written into TX (M01).
- Open question: OD-16.

### SM-D15 PROPOSAL: Command-Line Options

| Field             | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | PROPOSAL, not approved. The binary is named `socket-mux` and takes the grammar `socket-mux [GLOBAL OPTIONS] tx [TX OPTIONS]` or `socket-mux [GLOBAL OPTIONS] rx [RX OPTIONS]`, with the subcommands spelled `tx` and `rx`, plus `socket-mux [GLOBAL OPTIONS] tx-pat [TX-PAT OPTIONS]` with the spelling `tx-pat`, and command options after the subcommand. Global options are accepted before and after the subcommand; that part is decided. The option set, short forms, defaults, constraints, precedence, invalid combinations and exit statuses in the [specification](spec.md#command-line), independent of any argument-parsing library (the command-line part of OD-9). The `tx` `INPUT` operand is `SM-D16` and the `tx-pat` `--seed` option is `SM-D17`. |
| Rationale         | Global options carry the settings both sides must agree on (N, base port) or share (logging, UDP buffer size); command options carry settings only one side uses. Command options stay after the subcommand, and early usage errors stop a misconfigured side before it opens a socket.                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Alternatives      | Global options only before the subcommand.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Rejection reason  | Global options also work after the subcommand, as decided.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Affected surfaces | SM-P01, SM-P02, SM-P08, SM-P10; CLI                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |

- User need: one program runs either side (G2), and N is set on its command line (G5).
- `SM-D15-REQ1`: the grammar in the Proposal row above, with the options in the [specification](spec.md#command-line).
- Prospective acceptance `SM-D15-AC1`: each invalid combination exits with status 2 and binds no port, and `--help` and `--version` exit with status 0 (M11 to M14).
- Open question: OD-9; configuration sources other than the command line remain open.

### SM-D16 PROPOSAL: TX Input Operand

| Field             | Value                                                                                                                                                                                                                                                                                                                                                                             |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | PROPOSAL, not approved. `tx` takes an optional `INPUT` operand after its options. Absent, `tx` accepts one TCP connection at `--listen`. `-` reads standard input. Any other value is a file path. `--listen` with an `INPUT` operand, or more than one `INPUT` operand, is a usage error (OD-17). End of file on standard input or the file is end of stream, as a TCP close is. |
| Rationale         | A `-` or path operand is the netcat and Unix convention, and keeping TCP as the default keeps the current `tx` invocation unchanged.                                                                                                                                                                                                                                              |
| Alternatives      | An `--input <PATH or ->` option; standard input as the default when no option is given.                                                                                                                                                                                                                                                                                           |
| Rejection reason  | An option is longer to type for the common file case; a standard-input default makes a `tx` started from a terminal wait for typed input instead of a TCP client.                                                                                                                                                                                                                 |
| Affected surfaces | SM-P03, SM-P11; `--listen`; CLI                                                                                                                                                                                                                                                                                                                                                   |

- User need: `tx` takes a file or a pipe like netcat, and RX reassembles it on the remote end (G1).
- `SM-D16-REQ1`: the operand in the Proposal row above.
- Prospective acceptance `SM-D16-AC1`: in the lossless loop, RX's output equals the file given to `tx`, and equals the bytes piped into `tx -`; `--listen` with an operand exits with status 2 and binds no port. No test-matrix row covers it yet.
- Open question: OD-17.

### SM-D17 PROPOSAL: TX-PAT Seed Option

| Field             | Value                                                                                                                                                                                             |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | PROPOSAL, not approved. `tx-pat --seed <u64>` (short `-s`) sets a fixed seed. Without it, `tx-pat` draws a random 64-bit seed at start and logs it at `info`, so any run can be repeated (OD-18). |
| Rationale         | A fixed seed makes the pattern reproducible for validation; logging the random seed lets a failing random run be replayed.                                                                        |
| Alternatives      | A seed given as a string and hashed; no logging of the random seed.                                                                                                                               |
| Rejection reason  | A hashed string adds a second seed form to agree on; an unlogged random seed makes a failing run impossible to replay.                                                                            |
| Affected surfaces | SM-P10, SM-P12; CLI                                                                                                                                                                               |

- User need: the pattern loop is repeatable on demand and random by default (G6).
- `SM-D17-REQ1`: the option in the Proposal row above.
- Prospective acceptance `SM-D17-AC1`: two `tx-pat` runs with the same `--seed` emit the same pattern bytes; a run without `--seed` logs a seed that, given back with `--seed`, reproduces its pattern. No test-matrix row covers it yet.
- Open question: OD-18.

### SM-D18 PROPOSAL: Self-Locating Adler Pattern Blocks

| Field             | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | PROPOSAL, not approved. The pattern is a sequence of 4096-byte blocks. Block i holds i as an 8-byte big-endian index, then 4084 bytes from a pseudorandom generator keyed by the seed and i, then the Adler-32 checksum of the first 4092 bytes. A validator reading RX's output checks each block's Adler-32, and with the seed also regenerates its body. After a loss gap it slides one byte at a time until a 4096-byte window has a matching Adler-32 and, with the seed, a body equal to the regeneration for its index; it then reports the skipped byte count and continues. The validator is a separate checker that reads RX's TCP output; RX does not change (OD-19, OD-20). |
| Rationale         | The index and checksum in every block let validation find its place again after any loss, adversarial loss included, without trusting RX's offsets. The Adler-32 alone checks integrity when the seed was random and not recorded.                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Alternatives      | An unframed generator stream located by the header stream offset; RX validating the pattern itself.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Rejection reason  | RX's output skips loss gaps (`SM-D13`), so the validator does not see stream offsets; validation inside RX would check RX with its own code.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| Affected surfaces | SM-P13; `tx-pat`; verification loop                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

- User need: the review cycle validates RX against a known pattern and recovers after adversarial UDP loss (G3, G6).
- `SM-D18-REQ1`: the block format and validator in the Proposal row above.
- Prospective acceptance `SM-D18-AC1`: with a fixed seed and no loss, every block validates; with any set of dropped datagrams, the validator recovers within one block after each gap and every delivered block that is whole validates. No test-matrix row covers it yet.
- Assumption: 4096 bytes is a starting block size, not measured against the payload size (OD-3).
- Open questions: OD-19; OD-20; whether and how `tx-pat` ends (OD-21).

### SM-D21 PROPOSAL: Global Timeout Rules

| Field             | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | PROPOSAL, not approved. The global timeout of design SM-D19 is spelled `--timeout SECONDS`, a decimal number above 0 with default 10, accepted before or after every subcommand like the other global options. The timer starts when RX receives its first datagram and restarts on every datagram on any leg; before the first datagram RX waits without a limit. When it expires, RX declares every outstanding gap lost, delivers what follows each gap, logs each gap and one line saying the timeout ended the stream, closes its output and exits with status 0. TX and `tx-pat` accept the option and do not use it. |
| Rationale         | Restarting on any datagram ends the stream only when all legs are silent, so a slow but live sender is not cut off; waiting for the first datagram lets RX start before TX.                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Alternatives      | A timer per leg; a timer that starts when RX starts; a separate RX-only option.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Rejection reason  | A per-leg timer fires on a dead leg while the others still carry data; a timer from start ends a stream whose sender has not started yet; Tomas asked for a global option.                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Affected surfaces | Global options; RX output; SM-P09                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |

- User need: RX ends the stream even when the end-of-stream datagram is lost (G3).
- `SM-D21-REQ1`: the timer rules and spelling in the Proposal row above.
- Prospective acceptance `SM-D21-AC1`: with a dead leg that carries the end-of-stream datagram, RX closes its output within the timeout plus a small slack and reports exactly the dropped datagrams (rows M09, M10).
- Open questions: the timer's relation to the reorder timeout (OD-5).

### SM-D22 PROPOSAL: check-pat Checker Subcommand

| Field             | Value                                                                                                                                                                                                                                                                                                                                       |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposal          | PROPOSAL, not approved. The pattern validator of SM-D18 is a `check-pat [--seed U64]` subcommand of `socket-mux`. It reads RX's output on standard input, prints one line with the counts of valid and corrupt blocks, resynchronizations, lost bytes and trailing bytes, and exits with status 0 when no block is corrupt and 1 otherwise. |
| Rationale         | Operators who run `tx-pat` get a validator in the same binary, on every platform.                                                                                                                                                                                                                                                           |
| Alternatives      | A separate checker program; only the verification loop's own checker.                                                                                                                                                                                                                                                                       |
| Rejection reason  | None recorded; the subcommand was moved to the roadmap instead of the specified surface.                                                                                                                                                                                                                                                    |
| Affected surfaces | SM-P13; CLI                                                                                                                                                                                                                                                                                                                                 |

- User need: validate RX's output against the pattern without extra tools (G6).
- `SM-D22-REQ1`: the subcommand in the Proposal row above.
- Prospective acceptance `SM-D22-AC1`: on clean, lossy, truncated and corrupted pattern streams, its counts equal those of an independent checker.
- Open questions: OD-20.

### SM-R1 PROPOSAL: Pinned Per-Leg OS Threads

- User need: TX and RX move one stream at near line rate; per-packet syscall and CPU cost is the limit, not the wire [speculative synthesis, [max bandwidth](research/max-bandwidth.md)].
- `SM-R1-REQ1`: each lane runs on its own `std::thread`, pinned to the core that services its RSS queue (`core_affinity`, or SetThreadAffinityMask and sched_setaffinity through windows-sys and libc), with blocking or overlapped I/O and no async runtime on the hot path.
- `SM-R1-REQ2`: tokio, if used at all, runs only the control plane.
- Rationale: one core is what caps single-stream iperf3, so thread-per-lane beats one current_thread runtime [speculative]; single-stream iperf3 reaches about 3 Gbps and multi-thread iperf3 gets near line rate.
- Prospective acceptance `SM-R1-AC1`: per-layer measurement (`SM-R9`) shows the lane threads, not one runtime core, set the send and receive rate.
- Applicability: revises the tokio current_thread pick in `SM-D8` for the data path.
- Open question: thread-per-lane or the `SM-D8` runtime?

### SM-R2 PROPOSAL: quinn-udp For GSO, GRO And USO

- User need: batched UDP I/O on both platforms without writing each platform's batching by hand ([prior art](research/prior-art.md), item 3).
- `SM-R2-REQ1`: TX and RX use `quinn-udp` directly for GSO (UDP_SEGMENT), GRO and recvmmsg on Linux and for USO (UDP_SEND_MSG_SIZE) on Windows [proven, docs.rs].
- Assumption: URO (UDP_RECV_MAX_COALESCED_SIZE) stays off, as quinn-udp leaves it off by default on Windows after failures with WSL installed; it needs Windows 11 24H2 and NIC support [proven, quinn PR 2092].
- Prospective acceptance `SM-R2-AC1`: the batched path builds on Linux, macOS and Windows (SM-D7).
- Applicability: makes the optional "quinn-udp for batched I/O later" in `SM-D8` the batched I/O path.
- Open question: whether quinn-udp alone covers the `SM-R3` buffer layout.

### SM-R3 PROPOSAL: Win64 USO 64 KB Batches

- User need: Windows TX sends at 10G+ without one syscall per datagram.
- `SM-R3-REQ1`: TX calls `WSASetUdpSendMessageSize` (UDP_SEND_MSG_SIZE) and sends 64 KB buffers of MSS-sized segments.
- `SM-R3-REQ2`: each segment is its own datagram carrying its own header (`SM-D10`) inside its MSS bytes, so the buffer is laid out as [hdr|payload]×k; unless the driver has SubMssFinalSegmentSupported, the buffer length is a multiple of the MSS [proven, MS docs].
- Platform: Windows 10 2004 or Server 2004 (build 19041) or later, and NIC and driver USO support checked with `Get-NetAdapterUso` and turned on with `Enable-NetAdapterUso` [proven, MS docs].
- Evidence: one answer reports 2.3 Mpps with UDP_SEND_MSG_SIZE on 64 KB buffers [vendor-claimed]; no independent Windows USO throughput number was found.
- Prospective acceptance `SM-R3-AC1`: UDP send measured from Windows to Linux (`SM-R9` layer 2) with USO on exceeds the same run with USO off.
- Applicability: USO is not applied to loopback [proven], so a loopback back-to-back loop cannot show it.
- Open question: which NIC and driver the instrument PC has.

### SM-R4 PROPOSAL: Linux GRO Receive And Lane Steering

- User need: Linux RX receives every lane at line rate without receive-buffer drops.
- `SM-R4-REQ1`: one socket per lane port, each on its own thread; `ethtool -N` n-tuple rules steer dst-port base+k to queue k, IRQ affinity puts queue k on core k, and lane k's receiver thread runs on core k.
- `SM-R4-REQ2`: each lane socket sets UDP_GRO and reads with recvmmsg, splitting each coalesced read by the `UDP_GRO` cmsg gso_size; TX keeps all segments equal-size so GRO coalesces them [proven behavior; gain vendor-claimed].
- `SM-R4-REQ3`: SO_RCVBUF of tens of MB with `rmem_max` raised to allow it, and `ethtool -G rx` at the maximum; buffer size is the top cause of drops (UdpRcvbufErrors) [proven, TUM paper].
- Assumptions: SO_REUSEPORT and BPF steering are not needed because each lane has its own port [speculative]; without n-tuple support RSS hashes the 4-tuple and lanes may share a queue, which `ethtool -S` shows [speculative].
- Prospective acceptance `SM-R4-AC1`: during a measured run (`SM-R9` layer 5), per-queue packets in `ethtool -S` show lane k on queue k, and Udp RcvbufErrors in `/proc/net/snmp` stays at zero.
- Applicability: needs NIC n-tuple support, which is common on 10G+.
- Open questions: busy polling (`napi_defer_hard_irqs` with `gro_flush_timeout`, or SO_BUSY_POLL) trades CPU for fewer IRQs [proven, kernel docs] with unmeasured gain here; io_uring multishot recvmsg has no UDP number against recvmmsg with GRO.

### SM-R5 PROPOSAL: Slab Pool And Reassembly Ring

- User need: TX and RX carry the stream without a second copy and without unbounded memory.
- `SM-R5-REQ1`: TX reads TCP into a fixed pool of 64 KB slabs (BytesMut, or a pre-allocated arena), carves each slab into k segments of [header | payload] with header room reserved at write time, and hands each slab to lane (seq mod N) as one batched send.
- `SM-R5-REQ2`: RX places each slice in a ring indexed by seq mod R, with R ≥ 2 × (maximum reorder depth × N); the consumer drains contiguous sequence numbers and declares a gap lost after a timeout, since nothing is retransmitted.
- `SM-R5-REQ3`: water levels on slab-pool free count and ring occupancy apply backpressure by not reading the TCP socket.
- Evidence: spead2's docs call a memory pool "the single most important tool" [proven]; the slab and ring design is Scout's [speculative].
- Prospective acceptance `SM-R5-AC1`: with the reassembler alone fed a replayed or generated stream (`SM-R9` layer 4), memory stays within the pool and ring sizes and output matches the expected loss.
- Open questions: the loss timeout (OD-5); whether backpressure on the TCP side is the answer to flow control (OD-7).

### SM-R6 PROPOSAL: Jumbo MTU End To End

- User need: fewer datagrams per second for the same rate.
- `SM-R6-REQ1`: MTU 9000 on both ends and on every hop between them.
- Rationale: 10 Gbit/s is about 830k datagrams/s at 1472 B payload and about 140k/s at 9000 B, about 6× fewer [arithmetic; gain speculative]; spead2 gives jumbo-frame guidance ([prior art](research/prior-art.md), item 1).
- Prospective acceptance `SM-R6-AC1`: the wire baseline (`SM-R9` layer 3) runs with and without jumbo frames on the target path.
- Applicability: needs a switch and NIC path that carries jumbo frames.
- Open question: the payload size (OD-3).

### SM-R7 PROPOSAL: Larger Default UDP Buffer

- User need: TX and RX run without loss from socket buffers at their defaults.
- `SM-R7-REQ1`: the default of `--udp-buffer` is larger than the operating-system default that the specification's Global Options give it (`SM-D15`).
- Evidence: the verification harness's relay and loss runs failed at operating-system default buffers; QUIC measurements on 10G links found default buffers too small by 10× or more [proven, TUM paper]; the research proposes SO_RCVBUF of tens of MB on Linux and SO_SNDBUF of several MB on Windows [speculative].
- Prospective acceptance `SM-R7-AC1`: the lossless and relay rows of the test matrix (M01 to M04) pass with the default `--udp-buffer`, with no loss gap logged.
- Applicability: on Linux SO_RCVBUF is capped by `rmem_max` (`SM-R4-REQ3`).
- Open question: the default value.

### SM-R8 PROPOSAL: Deferred Kernel-Bypass Paths

- User need: headroom past the socket path, if it falls short.
- `SM-R8-REQ1`: socket-mux defers RIO, io_uring zcrx and AF_XDP until measurement shows the socket path short of line rate.
- RIO (registered buffers and completion queues, through the windows-sys RIO function table) ranks last: its 2012 gains were about 1.3× to 2.5× [proven, old], a newer report found no gain [proven, anecdotal], msquic removed its RIO datapath [vendor-claimed], and no maintained RIO crate was found.
- io_uring zcrx (Linux 6.15+) is TCP-oriented and supports only specific Broadcom and Mellanox NICs; UDP support was not found [proven docs, vendor-claimed perf].
- AF_XDP zero-copy needs driver support (i40e yes; ice and mlx5 depend on configuration) and bypasses the UDP stack, so socket-mux would parse headers itself [proven]; Rust options are `xsk-rs` or `aya` [not version-checked].
- Applicability: all three are NIC or driver dependent; AF_XDP is Linux only apart from XDP for Windows, whose receive is reported slow [vendor-claimed].
- Open question: whether per-layer measurement (`SM-R9`) shows the socket path short of line rate.

### SM-R9 PROPOSAL: Per-Layer Measurement Plan

- User need: each throughput prospect is judged by measurement at its own layer.
- `SM-R9-REQ1` loopback intake: a SCPI-simulator writer feeds the mux reader, which discards the data; record GB/s and CPU per core.
- `SM-R9-REQ2` UDP send on Windows: the mux with a synthetic source sends only, to the Linux box; compare with multi-thread iperf3 `-u -b 0 -l 1472/8972`.
- `SM-R9-REQ3` wire and NIC: an iperf3 TCP and UDP multi-stream baseline, with and without jumbo frames and USO.
- `SM-R9-REQ4` reassembly on Linux: a replayed pcap or a local generator feeds the reassembler alone, with divan for the ring.
- `SM-R9-REQ5` counters on Windows: `Get-NetAdapterStatistics` (OutboundDiscardedPackets, ReceivedDiscardedPackets); perfmon UDPv4 "Datagrams No Port/sec", "Datagrams Received Errors/sec" and "Datagrams Sent/sec"; Network Interface "Packets Outbound Discarded" and "Output Queue Length"; Processor "% Interrupt Time" and "% DPC Time" per core; `Get-NetAdapterRss` and `Get-NetAdapterUso`.
- `SM-R9-REQ6` counters on Linux: `ethtool -S` (rx_missed, rx_no_buffer, per-queue packets); `/proc/net/snmp` Udp InErrors and RcvbufErrors; `/proc/net/softnet_stat` (dropped, time_squeeze); `nstat -az | grep -i udp`; `/proc/interrupts` per queue; `mpstat -P ALL` for %soft.
- Assumption: the counter names are from Scout's knowledge and were not re-checked [speculative]; batching starting points are 64 KB per send call (UDP_SEGMENT allows at most 64 segments [proven, Cloudflare]) and 16 to 64 messages per recvmmsg, tuned from measurement [speculative].
- Prospective acceptance `SM-R9-AC1`: each layer reports its rate and the counters above for the same run.
- Open question: the measurement hosts and NICs, which the build and test targets (OD-11) do not yet name.

### SM-W1 Wanted: /goal Optimization Loop For Latency And Throughput

- User need: a /goal optimization loop for latency and throughput, wanted after a stable MVP release.
- Outcome: a /goal optimization loop that improves socket-mux latency and throughput.
- Sequencing: the loop follows a stable MVP release.
- Relation: the per-layer measurement plan `SM-R9` names the rates and counters per layer that such a loop could measure.
- Unspecified: the loop's mechanics, its latency and throughput metrics and targets, and what makes an MVP release stable. No requirement or acceptance criterion is drafted.

## Dependencies

`SM-D8` and `SM-R1` propose different data-path runtimes; Tomas decides between them under OD-1. `SM-R2` turns the optional quinn-udp of `SM-D8` into the batched path that `SM-R3` and `SM-R4` use. `SM-R3` puts one `SM-D10` header in each USO segment. `SM-R4` needs equal-size segments from TX and the receive buffers of `SM-R7`. `SM-R5` depends on the sequence number of `SM-D10` and bears on OD-5 and OD-7. `SM-R6` bears on the payload size (OD-3). `SM-R7` changes a default that `SM-D15` proposes. `SM-D13` uses the sequence and stream offset of `SM-D10`. `SM-D15` carries the defaults of `SM-D12`, and its grammar carries the operand of `SM-D16` and the option of `SM-D17`. `SM-D18` uses the seed of `SM-D17` and validates the output that `SM-D13` produces under loss. `SM-D21` fills in the timer that design SM-D19 decided and bears on OD-5. `SM-D22` is the validator that `SM-D18` describes. `SM-W1` waits on a stable MVP release and relates to `SM-R9`. `SM-R8` waits on `SM-R9`, and `SM-R1` to `SM-R7` are tuned from it.

## Readiness

A prospect is ready for a Change when its open question is decided, its behavior is in the [specification](spec.md) without the PROPOSAL mark, and its mechanism is a current decision in the [design](design.md). Readiness does not activate work.

## Boundaries

Outcomes are in the [goals](goals.md), current behavior in the [specification](spec.md), current mechanism and decisions in the [design](design.md), and background research in [research](research/). When a prospect is decided, the decision is drafted into its owner under the same ID and this roadmap records that disposition. This roadmap records no execution or release history.
