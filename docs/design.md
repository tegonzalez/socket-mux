---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-design]; otherwise, do not modify."
---

# socket-mux Design

## Purpose

Describe the intended architecture of socket-mux for authors who implement, extend or reconsider it: one program that, in TX or RX mode, carries one input stream, or a generated pattern, over N UDP lanes and reconstitutes it. The [specification](spec.md) owns consumer-visible behavior and terminology; this design owns mechanism, rationale and design decisions. Proposals awaiting a decision are in the [roadmap](roadmap.md).

## Module Inventory

No module is realized in the current baseline: the project holds documentation only. The inventory has no rows.

## Architecture

- **Program.** One program has two modes, TX and RX, exposed as CLI subcommands: TX and RX subcommands and a `tx-pat` subcommand (SM-D1). Its options, defaults, precedence and exit statuses are open (OD-9, OD-14).
- **TX path.** TX reads one input and splinters its byte stream efficiently into N UDP lanes, each on a different port, selected from a base port incremented by one by default (SM-D4). Every datagram carries a header (SM-D2), whose layout is open (OD-12). The policy that assigns slices to lanes is open (OD-4).
- **TX input.** The `tx` subcommand reads one TCP connection on localhost, standard input (`-`) or a file, like netcat (SM-D1); how the operator selects among them is open (OD-17). The `tx-pat` subcommand instead generates a continuous pseudorandom "adler pattern" from a random seed, or from a fixed seed set by an option (SM-D1); the pattern format and the seed option are open (OD-18, OD-19).
- **RX path.** RX listens "on a particular port plus one with a default number of slices" (SM-D5), on base port to base port + N - 1 (SM-D11). It reads each datagram header, sequences slices on reconstitution, determines packet loss and the length of that loss, and reconstitutes the original stream. Where RX delivers that stream (OD-16) and what it emits where it detects loss (OD-15) are open.
- **Loss.** Neither mode retransmits, so the channel is lossy (SM-D3).
- **Seam.** The UDP datagram, a header followed by slice data, is the one seam between TX and RX; this project owns both sides. The header layout (OD-12) and the payload size (OD-3) are undecided, and no contract document exists.
- **Verification loop.** TX and RX can be placed back to back in a verification loop (SM-D6). With `tx-pat` as TX, RX's output is validated against the fixed-seed pattern, and validation recovers after any UDP loss; the resynchronization mechanism and the validator are open (OD-19, OD-20).
- **End of stream.** RX delivers strictly in sequence order and ends the stream at the end-of-stream datagram, which it acts on only when it is next in sequence. The global timeout is the tolerance after which RX declares a missing sequence number lost (SM-D19). What ends a stream whose end-of-stream datagram is lost with no datagram after it is open (OD-22).
- **I/O model.** TX and RX are event-driven (SM-D20): each task waits on socket readiness, I/O completion, a channel or a timer deadline, and nothing spins or wakes on a fixed interval.
- **Platforms.** One code base targets Linux, macOS and Windows, and every dependency is cross-platform (SM-D7).

```text
TCP (localhost), stdin, file or pattern -> TX -> UDP lane 1 .. UDP lane N (N >= 3) -> RX -> original stream
```

## Design Decisions

Status `current` marks the intended design.

### SM-D1 One Program With TX And RX Subcommands

| Field             | Value                                                                                                                                                                                                                                                                                                                                                         |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Decision          | The program has two modes, transmit (TX) and receive (RX), exposed as a CLI with TX and RX subcommands, handling both sides. TX also has a `tx-pat` subcommand whose input is a generated pseudorandom pattern instead of an input stream. The TX subcommand reads its stream from one TCP connection on localhost, from standard input (`-`) or from a file. |
| Rationale         | One program handles both sides of the link. Standard input and file input make TX act like a netcat that splinters its input and allows reassembly on the remote end; the pattern lets RX's output be validated against a known pattern.                                                                                                                      |
| Alternatives      | None recorded.                                                                                                                                                                                                                                                                                                                                                |
| Rejection reason  | None recorded.                                                                                                                                                                                                                                                                                                                                                |
| Affected surfaces | SM-P01, SM-P02, SM-P03, SM-P10, SM-P11                                                                                                                                                                                                                                                                                                                        |
| Status            | current                                                                                                                                                                                                                                                                                                                                                       |

### SM-D2 Header Per UDP Datagram

| Field             | Value                                                                                                                                    |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Decision          | Every UDP datagram carries a header for sequencing slices on reconstitution and for determining packet loss and the length of that loss. |
| Rationale         | RX needs the header to sequence slices and to measure loss.                                                                              |
| Alternatives      | No per-datagram header.                                                                                                                  |
| Rejection reason  | Without a header RX cannot sequence slices or determine packet loss and its length.                                                      |
| Affected surfaces | SM-P05; TX path; RX path                                                                                                                 |
| Status            | current                                                                                                                                  |

### SM-D3 No Retransmission

| Field             | Value                                                              |
| ----------------- | ------------------------------------------------------------------ |
| Decision          | There is no retransmission at this stage, so the channel is lossy. |
| Rationale         | None recorded.                                                     |
| Alternatives      | Retransmission.                                                    |
| Rejection reason  | Not at this stage.                                                 |
| Affected surfaces | TX path; RX path; SM-P07                                           |
| Status            | current                                                            |

### SM-D4 TX Splinters Over N Ports

| Field             | Value                                                                                                                     |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Decision          | TX splinters into N lanes efficiently, each on a different port, selected from a base port incremented by one by default. |
| Rationale         | None recorded.                                                                                                            |
| Alternatives      | None recorded.                                                                                                            |
| Rejection reason  | None recorded.                                                                                                            |
| Affected surfaces | SM-P04, SM-P08                                                                                                            |
| Status            | current                                                                                                                   |

### SM-D5 RX Reconstitutes The Stream

| Field             | Value                                                                                                                                                              |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Decision          | RX listens "on a particular port plus one with a default number of slices" and reconstitutes the original stream into a single stream. SM-D11 defines those ports. |
| Rationale         | None recorded.                                                                                                                                                     |
| Alternatives      | None recorded.                                                                                                                                                     |
| Rejection reason  | None recorded.                                                                                                                                                     |
| Affected surfaces | SM-P06, SM-P07                                                                                                                                                     |
| Status            | current                                                                                                                                                            |

### SM-D6 Back-To-Back Verification Loop

| Field             | Value                                                        |
| ----------------- | ------------------------------------------------------------ |
| Decision          | TX and RX can be placed back to back in a verification loop. |
| Rationale         | None recorded.                                               |
| Alternatives      | None recorded.                                               |
| Rejection reason  | None recorded.                                               |
| Affected surfaces | TX path; RX path                                             |
| Status            | current                                                      |

### SM-D7 Cross-Platform Dependencies

| Field             | Value                                                                                                                                          |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Decision          | socket-mux is a Windows project and is cross-platform with Linux and Mac; every crate it depends on is compatible with Linux, Mac and Windows. |
| Rationale         | The tool is cross-platform.                                                                                                                    |
| Alternatives      | None recorded.                                                                                                                                 |
| Rejection reason  | None recorded.                                                                                                                                 |
| Affected surfaces | All dependencies; build targets (OD-11)                                                                                                        |
| Status            | current                                                                                                                                        |

### SM-D11 RX Listen Ports

| Field             | Value                                                                                                                                                                                                                                |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Decision          | Lanes are indexed 0 to N - 1, and lane k uses UDP port base port + k on both TX and RX. "A particular port plus one with a default number of slices" means RX listens on base port to base port + N - 1, the same ports TX sends to. |
| Rationale         | TX uses base port, base port + 1, ..., base port + N - 1 (SM-D4), so RX binding the same range needs only one shared base port and N.                                                                                                |
| Alternatives      | RX listens on one port, the base port plus one.                                                                                                                                                                                      |
| Rejection reason  | One port cannot receive N lanes that TX sends to N different ports.                                                                                                                                                                  |
| Affected surfaces | SM-P06; `--base-port`, `--streams`, `--bind`                                                                                                                                                                                         |
| Status            | current                                                                                                                                                                                                                              |

### SM-D19 Timeout Is The Inter-Packet Tolerance

| Field             | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Decision          | RX delivers strictly in sequence order. The global timeout option, default 10 s and tunable, is RX's inter-packet delay tolerance: RX keeps a monotonic arrival timestamp for each datagram it holds and declares a missing sequence number lost only when a later held datagram has waited longer than the timeout against the monotonic clock. The end-of-stream datagram carries a sequence number and is processed only when it is next in sequence or once the missing sequence numbers ahead of it have timed out. One timestamp per held datagram against one moving clock, not a timer per datagram. Tomas, 2026-10-21: "od-8: add a global timeout (option default ~10s?) that inject EOS" and "no packet is read out of sequence. so the 10 time out is not just for EOS, it is the inter-packet delay tolerance. it takes X (10) sections for an out of sequence packet to be “losted” .. this way, EOS has a seqno and is not processed until either it is in stream or the timeout expires for that packet .. note: this means this isn't implemented with multiple timers - it is a timestamp on rx per seqno packet against a moving clock (both monotonic)". The option's spelling and how RX waits for the earliest timestamp are PROPOSAL (SM-D21). |
| Rationale         | One tolerance governs loss and end of stream: RX never delivers out of sequence, a reordered datagram has the full timeout to arrive, and an end-of-stream datagram behind a lost datagram still ends the stream once the gap times out.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Alternatives      | A short separate reorder timeout plus an idle timer that ends the stream when no leg delivers; a timer per held datagram.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Rejection reason  | Tomas made the timeout the inter-packet tolerance and ruled out multiple timers (2026-10-21).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Affected surfaces | Global options; RX reordering and loss declaration; RX output; SM-P06, SM-P09                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Status            | current                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |

### SM-D20 Event-Driven I/O

| Field             | Value                                                                                                                                                                                                                                                                                                                              |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Decision          | socket-mux never busy-waits or polls. TX and RX wait on socket readiness or I/O completion, channels and timer deadlines, for example with tokio, so an idle process uses close to 0% CPU. Tomas, 2026-10-21: "no busy cpu. tokio or whatever needs to be done .. no polling anywhere we can use event driven. professional code". |
| Rationale         | A busy-polling RX held one CPU core at 100% while idle.                                                                                                                                                                                                                                                                            |
| Alternatives      | Busy-polling the sockets; sleeping and polling on an interval.                                                                                                                                                                                                                                                                     |
| Rejection reason  | Busy polling burns a core while idle; interval polling adds latency and still wakes the CPU. Tomas disallowed both.                                                                                                                                                                                                                |
| Affected surfaces | TX and RX I/O loops; RX's wait for the timeout                                                                                                                                                                                                                                                                                     |
| Status            | current                                                                                                                                                                                                                                                                                                                            |

## Failure Modes

- **Datagram loss.** RX determines packet loss and the length of that loss from the headers; nothing is retransmitted (SM-D3). What RX emits where it detects loss is open (OD-15). RX declares a slice lost rather than late once a later held datagram has waited longer than the timeout (SM-D19); whether a memory bound declares it sooner is open (OD-23).
- **Dead stream.** RX reports every gap a later datagram exposes, each once that datagram has waited longer than the timeout (SM-D19). If the dead stream carried the end-of-stream datagram and no datagram follows it, nothing is held to time out; what ends the stream then is open (OD-22).
