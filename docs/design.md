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
- **End of stream.** RX ends the stream at the end-of-stream datagram, or when the global timeout expires (SM-D19); the timeout also covers a lost end-of-stream datagram.
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

### SM-D19 Timeout Ends The Stream

| Field             | Value                                                                                                                                                                                                                                                                                                                                                                              |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Decision          | A global timeout option ends RX's stream when it expires: RX injects end of stream, delivers what it holds, reports the outstanding loss, and closes its output. The default is 10 s and is tunable. Tomas, 2026-10-21: "od-8: add a global timeout (option default ~10s?) that inject EOS". When the timer starts and restarts, and the option's spelling, are PROPOSAL (SM-D21). |
| Rationale         | When a dead stream carries the end-of-stream datagram, RX cannot otherwise learn that the stream ended.                                                                                                                                                                                                                                                                            |
| Alternatives      | RX waits until it is stopped.                                                                                                                                                                                                                                                                                                                                                      |
| Rejection reason  | A lost end-of-stream datagram would leave RX's output open forever.                                                                                                                                                                                                                                                                                                                |
| Affected surfaces | Global options; RX output; SM-P06, SM-P09                                                                                                                                                                                                                                                                                                                                          |
| Status            | current                                                                                                                                                                                                                                                                                                                                                                            |

### SM-D20 Event-Driven I/O

| Field             | Value                                                                                                                                                                                                                                                                                                                              |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Decision          | socket-mux never busy-waits or polls. TX and RX wait on socket readiness or I/O completion, channels and timer deadlines, for example with tokio, so an idle process uses close to 0% CPU. Tomas, 2026-10-21: "no busy cpu. tokio or whatever needs to be done .. no polling anywhere we can use event driven. professional code". |
| Rationale         | A busy-polling RX held one CPU core at 100% while idle.                                                                                                                                                                                                                                                                            |
| Alternatives      | Busy-polling the sockets; sleeping and polling on an interval.                                                                                                                                                                                                                                                                     |
| Rejection reason  | Busy polling burns a core while idle; interval polling adds latency and still wakes the CPU. Tomas disallowed both.                                                                                                                                                                                                                |
| Affected surfaces | TX and RX I/O loops; the reorder and global timeouts                                                                                                                                                                                                                                                                               |
| Status            | current                                                                                                                                                                                                                                                                                                                            |

## Failure Modes

- **Datagram loss.** RX determines packet loss and the length of that loss from the headers; nothing is retransmitted (SM-D3). What RX emits where it detects loss is open (OD-15). When RX declares a slice lost rather than late is open (OD-5).
- **Dead stream.** RX reports every gap a later datagram exposes. If the dead stream carried the end-of-stream datagram, the global timeout ends the stream (SM-D19).
