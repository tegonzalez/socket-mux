---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-spec]; otherwise, do not modify."
---

# socket-mux Specification

## Purpose

Define the consumer-visible behavior of socket-mux: its two modes and three subcommands, its command-line interface, the input TX accepts or generates, the UDP lanes and per-datagram header it uses, the stream it reconstitutes, its configurable lane count and the platforms it supports. The conformance subject is the socket-mux program running in either mode. "Must" marks a requirement. "PROPOSAL" marks behavior drafted for approval and not approved; it traces to a proposal in the [roadmap](roadmap.md). "Gap" marks behavior this specification does not define; a consumer relies on no particular behavior there, and each gap names its open question by its OD identifier.

## Terminology

| Term              | Meaning                                                                                                                                                               |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| TX                | Transmit mode, run by the `tx` or the `tx-pat` subcommand. It splinters its input into N UDP lanes.                                                                   |
| `tx` input        | The input of the `tx` subcommand: one TCP connection on localhost, standard input (`-`) or a file.                                                                    |
| Pattern           | The continuous pseudorandom pattern that `tx-pat` generates as its input, also called an "adler pattern".                                                             |
| Seed              | The value from which `tx-pat` generates the pattern: random by default, or a fixed seed set by an option.                                                             |
| RX                | Receive mode. It receives the N UDP lanes and reconstitutes the original stream.                                                                                      |
| N                 | The number of UDP lanes, which is also the number of slices.                                                                                                          |
| Lane              | One of the N UDP datagram flows from TX to RX; each uses a different port.                                                                                            |
| Slice             | A part of the original stream carried in one UDP datagram; the header sequences slices on reconstitution.                                                             |
| Base port         | The first port of the N lane ports; lane k (0 to N-1) uses base port + k.                                                                                             |
| Datagram header   | The header every UDP datagram carries.                                                                                                                                |
| Timeout T         | RX's one timeout, set by a global command-line option, default 10 s; the spelling `--timeout` is PROPOSAL (SM-D21). It is also called the inter-packet delay timeout. |
| Loss gap          | A run of consecutive slices RX does not receive.                                                                                                                      |
| Verification loop | TX and RX placed back to back; a pattern loop is `tx-pat` and RX placed back to back.                                                                                 |

## Model

TX reads one input and splinters it into N UDP lanes, each on a different port, with a header on every datagram. The `tx` subcommand reads one TCP connection on localhost, standard input (`-`) or a file, like netcat, and RX reassembles that stream on the remote end. The `tx-pat` subcommand behaves as `tx` does, except that its input is a pattern it generates continuously instead of an input stream. RX listens "on a particular port plus one with a default number of slices", uses the headers to sequence slices and to determine packet loss and the length of that loss, and reconstitutes the original stream. Neither mode retransmits, so the channel is lossy.

```text
TCP client (127.0.0.1:31100), standard input or file -> tx -> UDP base port .. base port + N - 1 -> RX -> TCP peer (127.0.0.1:31200)
generated pattern                                      -> tx-pat -> UDP base port .. base port + N - 1 -> RX -> TCP peer -> pattern validation
```

The ports in the diagram are the PROPOSAL defaults (SM-D12).

## Public Surface

| ID     | Kind                  | Visibility | Use mode                                  | Exact shape                                                                                                                                                                                                                | Trigger                          | Error condition                                                                                                   | Derivation                           |
| ------ | --------------------- | ---------- | ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| SM-P01 | CLI subcommand        | public     | invoked by operator                       | PROPOSAL (SM-D15): `socket-mux [GLOBAL OPTIONS] tx [TX OPTIONS]`; options in [Command Line](#command-line)                                                                                                                 | Operator starts transmit mode    | Usage error, exit status 2 (PROPOSAL)                                                                             | design SM-D1; roadmap SM-D15         |
| SM-P10 | CLI subcommand        | public     | invoked by operator                       | `tx-pat`: TX whose input is the pattern instead of an input stream; PROPOSAL (SM-D15, SM-D17): `socket-mux [GLOBAL OPTIONS] tx-pat [TX-PAT OPTIONS]`; options in [Command Line](#command-line)                             | Operator starts pattern transmit | Usage error, exit status 2 (PROPOSAL)                                                                             | design SM-D1; roadmap SM-D15, SM-D17 |
| SM-P02 | CLI subcommand        | public     | invoked by operator                       | PROPOSAL (SM-D15): `socket-mux [GLOBAL OPTIONS] rx [RX OPTIONS]`; options in [Command Line](#command-line)                                                                                                                 | Operator starts receive mode     | Usage error, exit status 2 (PROPOSAL)                                                                             | design SM-D1; roadmap SM-D15         |
| SM-P03 | TCP ingress           | public     | connected to by a local client            | One TCP connection on localhost, one of the three `tx` inputs, accepted by `tx` at `--listen`; PROPOSAL default `127.0.0.1:31100`                                                                                          | A local client connects to `tx`  | Bind failure: exit status 1 (PROPOSAL)                                                                            | roadmap SM-D12                       |
| SM-P11 | Stream ingress        | public     | read by `tx`                              | `-` reads standard input; a file path reads that file; either is a `tx` input in place of the TCP connection. How the operator selects TCP, `-` or a file is a gap (OD-17); PROPOSAL (SM-D16): an optional `INPUT` operand | `tx` starts with `-` or a file   | Open or read failure: exit status 1 (PROPOSAL)                                                                    | design SM-D1; roadmap SM-D16         |
| SM-P04 | UDP lane set          | public     | sent by TX, received by RX                | N UDP lanes on ports base port to base port + N - 1, TX sending lane k to `--remote` at base port + k; PROPOSAL defaults N = 4, base port 31000                                                                            | TX carries data from its input   | A datagram that does not arrive is declared lost under timeout rule A or rule B ([Semantics](#semantics), SM-D19) | design SM-D4, SM-D11; roadmap SM-D12 |
| SM-P05 | Datagram header       | public     | carried on every UDP datagram             | A header per UDP datagram that lets RX sequence slices on reconstitution and determine packet loss and the length of that loss; PROPOSAL layout in [Datagram Header](#datagram-header); payload size is a gap (OD-3)       | TX sends a datagram              | RX discards a datagram whose magic or version does not match (PROPOSAL)                                           | design SM-D2; roadmap SM-D10         |
| SM-P06 | RX listen             | public     | bound by RX                               | RX listens "on a particular port plus one with a default number of slices"; RX binds base port to base port + N - 1 (lane k on base port + k); PROPOSAL address option `--bind` (SM-D15)                                   | RX starts                        | Bind failure: exit status 1 (PROPOSAL)                                                                            | design SM-D5, SM-D11                 |
| SM-P07 | Reconstituted stream  | public     | produced by RX                            | The original stream, reconstituted from the N UDP lanes; PROPOSAL: RX connects out over TCP to `--connect`, default `127.0.0.1:31200`, and writes the stream in order, skipping loss gaps                                  | RX receives datagrams            | Connect failure: exit status 1 (PROPOSAL)                                                                         | design SM-D5; roadmap SM-D13, SM-D14 |
| SM-P08 | Setting: lane count   | public     | configured by operator                    | N is configurable with a minimum of 3, and its default is configurable; PROPOSAL: `--streams`, default 4                                                                                                                   | Either mode starts               | N below 3: usage error, exit status 2 (PROPOSAL)                                                                  | roadmap SM-D12, SM-D15               |
| SM-P09 | Diagnostic: loss gap  | public     | logged by RX                              | PROPOSAL: one `warn` log line per loss gap naming the lost slice count and byte length, plus running totals                                                                                                                | RX skips a loss gap              | None                                                                                                              | roadmap SM-D13                       |
| SM-P12 | Setting: pattern seed | public     | configured by operator                    | `tx-pat` generates the pattern from a random seed by default; an option sets a fixed seed. PROPOSAL (SM-D17): `--seed`                                                                                                     | `tx-pat` starts                  | Gap (OD-18)                                                                                                       | roadmap SM-D17                       |
| SM-P13 | Pattern               | public     | generated by `tx-pat`, validated after RX | A continuous pseudorandom "adler pattern"; RX's reconstituted output can be validated against it from any point after a loss gap. PROPOSAL format and validator: SM-D18                                                    | `tx-pat` runs                    | Gap (OD-19, OD-20)                                                                                                | roadmap SM-D18                       |

### Command Line

#### Grammar

PROPOSAL (SM-D15): the binary name `socket-mux` and this grammar are not approved. socket-mux is a CLI with TX and RX subcommands and a `tx-pat` subcommand (SM-D1). PROPOSAL (SM-D16): `tx` takes an optional `INPUT` operand.

```text
socket-mux [GLOBAL OPTIONS] tx [GLOBAL OPTIONS | TX OPTIONS]... [INPUT]
socket-mux [GLOBAL OPTIONS] tx-pat [GLOBAL OPTIONS | TX-PAT OPTIONS]...
socket-mux [GLOBAL OPTIONS] rx [GLOBAL OPTIONS | RX OPTIONS]...
socket-mux --help | --version
```

Global options are accepted both before and after the subcommand, for every subcommand. PROPOSAL (SM-D15): command options appear after the subcommand. Both sides of one link must use the same `--streams` and `--base-port`.

#### Global Options

Every option in this table is PROPOSAL (SM-D15) except where the Trace column cites a current decision for the setting itself.

| Long           | Short | Value                                       | Default                  | Constraints                                                                                               | Trace                                                                |
| -------------- | ----- | ------------------------------------------- | ------------------------ | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `--streams`    | `-n`  | integer N                                   | 4                        | N >= 3; base port + N - 1 <= 65535                                                                        | N configurable, minimum 3, configurable default; default SM-D12      |
| `--base-port`  | `-p`  | integer port                                | 31000                    | 1 to 65535; base port + N - 1 <= 65535                                                                    | base port incremented by one; SM-D4, SM-D11, SM-D12                  |
| `--udp-buffer` | none  | integer bytes                               | operating-system default | >= 1; TX sets each UDP send buffer, RX each UDP receive buffer                                            | SM-D15                                                               |
| `--timeout`    | none  | seconds                                     | 10                       | > 0; the timeout T of timeout rules A and B ([Semantics](#semantics), SM-D19); spelling PROPOSAL (SM-D21) | one global RX timeout T, a CLI option, default 10 s; spelling SM-D21 |
| `--log-level`  | none  | `error`, `warn`, `info`, `debug` or `trace` | `info`                   | one of the listed values                                                                                  | SM-D15                                                               |
| `--verbose`    | `-v`  | none; repeatable                            | none                     | each use raises the level by one, up to `trace`                                                           | SM-D15                                                               |
| `--quiet`      | `-q`  | none; repeatable                            | none                     | each use lowers the level by one, down to `error`                                                         | SM-D15                                                               |
| `--help`       | `-h`  | none                                        | none                     | prints usage and exits with status 0                                                                      | SM-D15                                                               |
| `--version`    | `-V`  | none                                        | none                     | prints the version and exits with status 0                                                                | SM-D15                                                               |

#### TX Options

These are the options of `tx`. `tx` accepts `-` or a file as its input; the spelling below is PROPOSAL (SM-D16).

| Long            | Short | Value                   | Default                                               | Constraints                                                              | Trace                                                      |
| --------------- | ----- | ----------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------ | ---------------------------------------------------------- |
| `--listen`      | `-l`  | `addr:port`             | `127.0.0.1:31100`                                     | loopback address only; port 1 to 65535; used only when `INPUT` is absent | one TCP connection in on localhost; default SM-D12; SM-D16 |
| `--remote`      | `-r`  | host name or IP address | `127.0.0.1`                                           | resolves to one address; how the two sides agree on it is a gap (OD-10)  | SM-D15; gap OD-10                                          |
| `INPUT` operand | none  | `-` or a file path      | absent: `tx` accepts one TCP connection at `--listen` | `-` reads standard input; any other value names a file                   | accepts `-` or a file; spelling SM-D16; gap OD-17          |

#### TX-PAT Options

These are the options of `tx-pat`. The seed is random by default, and an option sets a fixed seed; the spelling below is PROPOSAL (SM-D17).

| Long       | Short | Value                   | Default     | Constraints                                                | Trace                                                                          |
| ---------- | ----- | ----------------------- | ----------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `--remote` | `-r`  | host name or IP address | `127.0.0.1` | as for `tx`                                                | SM-D15; gap OD-10                                                              |
| `--seed`   | `-s`  | unsigned 64-bit integer | random      | absent: `tx-pat` picks a random seed and logs it at `info` | random by default or a fixed seed set by an option; spelling SM-D17; gap OD-18 |

#### RX Options

| Long        | Short | Value       | Default           | Constraints                                                           | Trace          |
| ----------- | ----- | ----------- | ----------------- | --------------------------------------------------------------------- | -------------- |
| `--bind`    | `-b`  | IP address  | `127.0.0.1`       | a local address; `0.0.0.0` or `::` accepts datagrams from other hosts | SM-D11, SM-D15 |
| `--connect` | `-c`  | `addr:port` | `127.0.0.1:31200` | port 1 to 65535                                                       | SM-D14         |

#### Option Precedence

PROPOSAL (SM-D15): an option given on the command line overrides its default. A repeated value option takes its last value, including a global option given both before and after the subcommand; `-v` and `-q` count from both positions. `--log-level` sets the starting level, then each `-v` raises it and each `-q` lowers it by one, in any order. `--help` and `--version` take precedence over every other option and over a missing subcommand. Other configuration sources, such as a file or environment variables, are a gap (OD-9).

#### Invalid Combinations

PROPOSAL (SM-D15): each of these is a usage error with exit status 2 and no socket opened.

- No subcommand, or a subcommand other than `tx`, `tx-pat` or `rx`.
- A command option before the subcommand.
- An option of one subcommand with another subcommand; `--remote` belongs to both `tx` and `tx-pat`.
- More than one `INPUT` operand, or `--listen` together with an `INPUT` operand.
- `--streams` below 3, a port outside 1 to 65535, or base port + N - 1 above 65535.
- `--listen` with a non-loopback address.
- A missing, malformed or non-numeric option value.

#### Exit Status

PROPOSAL (SM-D15):

| Status | Meaning                                                                                                                                                                                                                                                                                                                                                       |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0      | The stream ended normally: `tx` after its input ended (its TCP client closed, or standard input or the file reached end of file) and the end-of-stream datagrams were sent, RX after it delivered the stream through end of stream, including a stream that timeout rule B ended (SM-D19); also `--help` and `--version`. Loss gaps do not change the status. |
| 1      | Runtime failure: a bind, connect, resolve, file open or read, or socket I/O error.                                                                                                                                                                                                                                                                            |
| 2      | Usage error, as listed under Invalid Combinations.                                                                                                                                                                                                                                                                                                            |

#### Usage Examples

Back-to-back verification loop on one host, with every PROPOSAL default (four terminals; `nc` stands for any TCP client and sink):

```sh
nc -l 127.0.0.1 31200 > received.bin    # sink for the reconstituted stream
socket-mux rx                           # binds UDP 127.0.0.1:31000-31003, connects to 127.0.0.1:31200
socket-mux tx                           # listens on 127.0.0.1:31100, sends to 127.0.0.1:31000-31003
nc 127.0.0.1 31100 < original.bin       # source stream
cmp original.bin received.bin           # equal when no loss gap was logged
```

The same loop with `tx` reading a file, then standard input, instead of a TCP client:

```sh
socket-mux tx original.bin              # reads the file
cat original.bin | socket-mux tx -      # reads standard input
```

A pattern loop with a fixed seed, so the validator can regenerate the pattern; the validator is a gap (OD-20):

```sh
nc -l 127.0.0.1 31200 > received.bin
socket-mux rx
socket-mux tx-pat --seed 42
```

Two hosts with eight lanes; RX at 192.0.2.10 delivers to a local service on port 8080:

```sh
socket-mux -n 8 rx --bind 0.0.0.0 --connect 127.0.0.1:8080       # on 192.0.2.10
socket-mux -n 8 tx --listen 127.0.0.1:18080 --remote 192.0.2.10    # on the sending host
```

Larger socket buffers and debug logging on both sides:

```sh
socket-mux -v --udp-buffer 4194304 rx
socket-mux -v --udp-buffer 4194304 tx
```

### Datagram Header

PROPOSAL (SM-D10). The header that every datagram carries is current (SM-D2); this layout is not approved. Multi-byte fields use network byte order (big-endian). The header is 22 bytes, followed by `payload length` bytes of slice data.

| Field          | Offset | Size (bytes) | Byte order | Purpose                                                                                                    |
| -------------- | ------ | ------------ | ---------- | ---------------------------------------------------------------------------------------------------------- |
| magic          | 0      | 2            | network    | Constant `0x534D` ("SM"); marks a socket-mux datagram.                                                     |
| version        | 2      | 1            | n/a        | Header format version; this layout is version 1.                                                           |
| flags          | 3      | 1            | n/a        | Bit 0 = end of stream; bits 1 to 7 are zero.                                                               |
| payload length | 4      | 2            | network    | Number of slice-data bytes after the header.                                                               |
| sequence       | 6      | 8            | network    | Global slice sequence number across all N lanes: 0 for the first slice, one more for each following slice. |
| stream offset  | 14     | 8            | network    | Byte offset of this slice's first byte in the original stream.                                             |

## Invariants

- N must be at least 3.
- Each lane must use a different port.
- Every UDP datagram must carry the datagram header.
- Neither mode retransmits a lost datagram.
- `tx` reads exactly one input: one TCP connection on localhost, standard input (`-`) or one file.
- `tx-pat` reads no input stream; it generates the pattern continuously and sends it under the same port and header rules as `tx`.
- `tx-pat` uses a random seed unless an option sets a fixed seed.
- RX's output from a pattern loop must be checkable against the pattern from any point after a loss gap, under any pattern of UDP datagram loss, including adversarial loss.
- RX delivers strictly in sequence order and never delivers a datagram out of sequence (SM-D19).
- RX holds no datagram longer than the timeout T.
- Neither mode busy-waits or polls. Every wait is event-driven (socket readiness, I/O completion or a timer deadline), so an idle TX or RX uses close to no CPU (SM-D20). Tomas, 2026-10-21: "no busy cpu. tokio or whatever needs to be done .. no polling anywhere we can use event driven. professional code".

## Semantics

- Ports (SM-D11): stream k, for k from 0 to N - 1, uses base port + k on both sides.
- Loss: RX determines packet loss and the length of that loss from the datagram headers. PROPOSAL (SM-D13): the sequence gap gives the lost slice count and the stream-offset gap gives the lost byte length; RX skips the lost bytes, keeps the order of the bytes it delivers, and logs the gap (SM-P09). RX declares a missing sequence number lost only under timeout rule A or rule B below (SM-D19); whether a bound on held datagrams declares it sooner is a gap (OD-22).
- Input: `tx` splinters its input across the N UDP streams, and RX reassembles it on the remote end, so `tx` and RX act like a netcat split over N UDP streams. Tomas, 2026-10-21: "expand tx command to accept a '-' or file to stream ie std input as the input so it act like a netcat which splinters and allows reassembly on remote end".
- Pattern: `tx-pat` generates "a continuous psuedorandom adler pattern (random (default) or option configured seed) instead of accepting tx stream to be split" (Tomas, 2026-10-21). The pattern's format, where Adler checksums appear in it and whether `tx-pat` ends are gaps (OD-19, OD-21).
- Pattern verification: `tx-pat` paired with RX is a verification loop. RX's reconstituted output is validated against the known pattern of a fixed seed, and validation recovers after any UDP loss, including adversarial loss. Tomas, 2026-10-21: "we can augment the review cycle to use tx-pat as the paired process of rx can be validated against the fixed adler pattern and recover even with adversarial packet loss of the udp". How the pattern allows resynchronization (OD-19) and which program validates it (OD-20) are gaps.
- End of stream (SM-D19): the end-of-stream datagram carries a sequence number. RX processes it only in sequence order, or when the timeout releases it under rule A or rule B below. Rule B therefore also ends a stream whose end-of-stream datagram was lost. PROPOSAL (SM-D10): `tx` marks its final datagram with the end-of-stream flag after its input ends, and RX closes its TCP output after delivering through that datagram.
- Timeout (SM-D19): RX has one timeout, T, and applies two rules with it.
  - Rule A: a run of missing sequence numbers is declared lost once T has passed since the arrival of the datagram just before the run, which is the received datagram with the highest sequence number below the run. Every sequence number in the run expires with it, and RX delivers on in sequence order from the next datagram it holds.
  - Rule B: once T has passed since the last datagram RX received, with no new datagram, RX declares every remaining gap lost, delivers everything it holds in sequence order, and ends the stream.
  - Worked example: datagrams 1, 2, 6 and 7 arrive at 0 s, and the stream continues to 10005 and 10006 at 10 s. With T = 10 s, at 10 s the inter-packet delay from 2 to 3 has expired, so 3 is lost, and 4 and 5 expire with it.
  - PROPOSAL (SM-D21): before the first datagram RX waits without a limit, and a gap at the start of the stream, with no datagram before it, is measured from the first datagram RX received.
- Verification loop: TX and RX can be placed back to back in a verification loop, with either `tx` or `tx-pat` as TX.

## Diagnostics

- Loss gap (SM-P09): PROPOSAL; trigger: RX skips a loss gap; facts: first missing sequence number, lost slice count, lost byte length, running totals.
- Timeout end of stream: PROPOSAL (SM-D21); trigger: timeout rule B ends the stream; facts: the timeout T and the running loss totals.
- Exit status: PROPOSAL; see [Exit Status](#exit-status).

## Boundaries

- Platforms: socket-mux runs on Linux, macOS and Windows.
- Retransmission is out of scope at this stage.
- Payload size (OD-3), slice assignment to streams (OD-4), timeouts on the TCP side (OD-5), a bound on held datagrams (OD-22), encryption and authentication (OD-6), flow control (OD-7), configuration sources other than the command line (OD-9), far-end addressing agreement (OD-10), build and test targets (OD-11), `tx` input selection (OD-17), the seed option (OD-18), the pattern format and resynchronization (OD-19), the pattern validator (OD-20) and the end of `tx-pat` (OD-21) are gaps.
