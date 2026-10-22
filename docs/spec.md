---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-spec]; otherwise, do not modify."
---

# socket-mux Specification

## Purpose

Define the consumer-visible behavior of socket-mux: its two modes and three subcommands, its command-line interface, the input TX accepts or generates, the UDP lanes and per-datagram header it uses, the stream it reconstitutes, its configurable lane count and the platforms it supports. The conformance subject is the socket-mux program running in either mode. "Must" marks a requirement. "Gap" marks behavior this specification does not define; and a consumer relies on no particular behavior there.

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
| Datagram size     | The size of every UDP datagram, header included, set by a global option.                                                                                              |
| Arena             | RX's fixed memory for the datagrams it holds out of order or has not yet written to its TCP output; its size is set by a global option.                               |
| Window W          | The number of datagrams the arena holds: arena size ÷ datagram size, rounded down.                                                                                    |
| Sequence width S  | The bit width of the sequence number, a build constant: 32 in normal builds, 8 or 16 in builds that test rollover.                                                    |
| Next              | The lowest sequence number RX has neither delivered nor declared lost.                                                                                                |
| Arrival time      | The time RX's application dequeues a datagram from its socket, read from a monotonic clock; not a kernel timestamp.                                                   |
| Verification loop | TX and RX placed back to back; a pattern loop is `tx-pat` and RX placed back to back.                                                                                 |

## Model

TX reads one input and splinters it into N UDP lanes, each on a different port, with a header on every datagram. The `tx` subcommand reads one TCP connection on localhost, standard input (`-`) or a file, like netcat, and RX reassembles that stream on the remote end. The `tx-pat` subcommand behaves as `tx` does, except that its input is a pattern it generates continuously instead of an input stream. RX listens "on a particular port plus one with a default number of slices", uses the headers to sequence slices and to determine packet loss and the length of that loss, and reconstitutes the original stream. Neither mode retransmits, so the channel is lossy.

```text
TCP client (127.0.0.1:31100), standard input or file -> tx -> UDP base port .. base port + N - 1 -> RX -> TCP peer (127.0.0.1:31200)
generated pattern                                      -> tx-pat -> UDP base port .. base port + N - 1 -> RX -> TCP peer -> pattern validation
```

The ports in the diagram are the defaults (SM-D12).

## Public Surface

| ID     | Kind                   | Visibility | Use mode                                  | Exact shape                                                                                                                                                                                                                                 | Trigger                          | Error condition                                                                                                         | Derivation                           |
| ------ | ---------------------- | ---------- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| SM-P01 | CLI subcommand         | public     | invoked by operator                       | PROPOSAL (SM-D15): `socket-mux [GLOBAL OPTIONS] tx [TX OPTIONS]`; options in [Command Line](#command-line)                                                                                                                                  | Operator starts transmit mode    | Usage error, exit status 2 (PROPOSAL)                                                                                   | design SM-D1; roadmap SM-D15         |
| SM-P10 | CLI subcommand         | public     | invoked by operator                       | `tx-pat`: TX whose input is the pattern instead of an input stream; PROPOSAL (SM-D15, SM-D17): `socket-mux [GLOBAL OPTIONS] tx-pat [TX-PAT OPTIONS]`; options in [Command Line](#command-line)                                              | Operator starts pattern transmit | Usage error, exit status 2 (PROPOSAL)                                                                                   | design SM-D1; roadmap SM-D15, SM-D17 |
| SM-P02 | CLI subcommand         | public     | invoked by operator                       | PROPOSAL (SM-D15): `socket-mux [GLOBAL OPTIONS] rx [RX OPTIONS]`; options in [Command Line](#command-line)                                                                                                                                  | Operator starts receive mode     | Usage error, exit status 2 (PROPOSAL)                                                                                   | design SM-D1; roadmap SM-D15         |
| SM-P03 | TCP ingress            | public     | connected to by a local client            | One TCP connection on localhost, one of the three `tx` inputs, accepted by `tx` at `--listen`; PROPOSAL default `127.0.0.1:31100`                                                                                                           | A local client connects to `tx`  | Bind failure: exit status 1 (PROPOSAL)                                                                                  | roadmap SM-D12                       |
| SM-P11 | Stream ingress         | public     | read by `tx`                              | `-` reads standard input; a file path reads that file; either is a `tx` input in place of the TCP connection. How the operator selects TCP, `-` or a file is a gap (OD-17); PROPOSAL (SM-D16): an optional `INPUT` operand                  | `tx` starts with `-` or a file   | Open or read failure: exit status 1 (PROPOSAL)                                                                          | design SM-D1; roadmap SM-D16         |
| SM-P04 | UDP lane set           | public     | sent by TX, received by RX                | N UDP lanes on ports base port to base port + N - 1, TX sending lane k to `--remote` at base port + k; default N = 1; PROPOSAL default base port 31000                                                                                      | TX carries data from its input   | A datagram that does not arrive is declared lost under timeout rule A or B or the arena limit ([Semantics](#semantics)) | design SM-D4, SM-D11; roadmap SM-D12 |
| SM-P05 | Datagram header        | public     | carried on every UDP datagram             | A header per UDP datagram that lets RX sequence slices on reconstitution and determine packet loss and the length of that loss; PROPOSAL layout in [Datagram Header](#datagram-header); each datagram is at most the datagram size (SM-P14) | TX sends a datagram              | RX discards a datagram whose magic or version does not match (PROPOSAL)                                                 | design SM-D2; roadmap SM-D10         |
| SM-P06 | RX listen              | public     | bound by RX                               | RX listens "on a particular port plus one with a default number of slices"; RX binds base port to base port + N - 1 (lane k on base port + k); PROPOSAL address option `--bind` (SM-D15)                                                    | RX starts                        | Bind failure: exit status 1 (PROPOSAL)                                                                                  | design SM-D5, SM-D11                 |
| SM-P07 | Reconstituted stream   | public     | produced by RX                            | The original stream, reconstituted from the N UDP lanes; PROPOSAL: RX connects out over TCP to `--connect`, default `127.0.0.1:31200`, and writes the stream in order, skipping loss gaps                                                   | RX receives datagrams            | Connect failure: exit status 1 (PROPOSAL)                                                                               | design SM-D5; roadmap SM-D13, SM-D14 |
| SM-P08 | Setting: lane count    | public     | configured by operator                    | N is configurable with a minimum of 1, and its default is configurable; default 1; PROPOSAL spelling: `--streams`                                                                                                                           | Either mode starts               | N below 1: usage error, exit status 2 (PROPOSAL)                                                                        | roadmap SM-D15                       |
| SM-P09 | Diagnostic: loss gap   | public     | logged by RX                              | One log line per loss gap naming its cause; PROPOSAL: at `warn`, also naming the lost slice count and byte length, plus running totals                                                                                                      | RX skips a loss gap              | None                                                                                                                    | roadmap SM-D13                       |
| SM-P12 | Setting: pattern seed  | public     | configured by operator                    | `tx-pat` generates the pattern from a random seed by default; an option sets a fixed seed. PROPOSAL (SM-D17): `--seed`                                                                                                                      | `tx-pat` starts                  | Gap (OD-18)                                                                                                             | roadmap SM-D17                       |
| SM-P13 | Pattern                | public     | generated by `tx-pat`, validated after RX | A continuous pseudorandom "adler pattern"; RX's reconstituted output can be validated against it from any point after a loss gap. PROPOSAL format and validator: SM-D18                                                                     | `tx-pat` runs                    | Gap (OD-19, OD-20)                                                                                                      | roadmap SM-D18                       |
| SM-P14 | Setting: datagram size | public     | configured by operator                    | The UDP datagram size, header included, is configurable; PROPOSAL: `--datagram-size`, default 1452                                                                                                                                          | Either mode starts               | Out of range: usage error, exit status 2 (PROPOSAL)                                                                     | design SM-D24; roadmap SM-D15        |
| SM-P15 | Setting: arena size    | public     | configured by operator                    | The total size of RX's arena; W = arena size ÷ datagram size. PROPOSAL: `--arena-size`, default 64 MiB                                                                                                                                      | RX starts                        | Out of range: usage error, exit status 2 (PROPOSAL)                                                                     | design SM-D24; roadmap SM-D15        |

### Command Line

#### Grammar

The binary is `socket-mux` with this grammar (SM-D15). socket-mux is a CLI with TX and RX subcommands and a `tx-pat` subcommand (SM-D1). `tx` takes an optional `INPUT` operand (SM-D16).

```text
socket-mux [GLOBAL OPTIONS] tx [GLOBAL OPTIONS | TX OPTIONS]... [INPUT]
socket-mux [GLOBAL OPTIONS] tx-pat [GLOBAL OPTIONS | TX-PAT OPTIONS]...
socket-mux [GLOBAL OPTIONS] rx [GLOBAL OPTIONS | RX OPTIONS]...
socket-mux --help | --version
```

Global options are accepted both before and after the subcommand, for every subcommand. PROPOSAL (SM-D15): command options appear after the subcommand. Both sides of one link must use the same `--streams`, `--base-port` and `--datagram-size`.

#### Global Options

The Trace column names the source of each setting.

| Long              | Short | Value                                       | Default                  | Constraints                                                                                               | Trace                                                                |
| ----------------- | ----- | ------------------------------------------- | ------------------------ | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `--streams`       | `-n`  | integer N                                   | 1                        | N >= 1; base port + N - 1 <= 65535                                                                        | N configurable with a configurable default; default 1, minimum 1     |
| `--base-port`     | `-p`  | integer port                                | 31000                    | 1 to 65535; base port + N - 1 <= 65535                                                                    | base port incremented by one; SM-D4, SM-D11, SM-D12                  |
| `--udp-buffer`    | none  | integer bytes                               | operating-system default | >= 1; TX sets each UDP send buffer, RX each UDP receive buffer                                            | SM-D15                                                               |
| `--timeout`       | none  | seconds                                     | 10                       | > 0; the timeout T of timeout rules A and B ([Semantics](#semantics), SM-D19); spelling PROPOSAL (SM-D21) | one global RX timeout T, a CLI option, default 10 s; spelling SM-D21 |
| `--datagram-size` | none  | integer bytes                               | 1452                     | header size + 1 to 65507; TX sends no larger datagram; RX discards a larger one (PROPOSAL)                | datagram size configurable (SM-P14); spelling and default SM-D15     |
| `--arena-size`    | none  | integer bytes                               | 67108864 (64 MiB)        | 1 <= W < 2^(S-1); RX uses it; TX accepts it and does not use it                                           | arena size configurable (SM-P15); spelling and default SM-D15        |
| `--log-level`     | none  | `error`, `warn`, `info`, `debug` or `trace` | `info`                   | one of the listed values                                                                                  | SM-D15                                                               |
| `--verbose`       | `-v`  | none; repeatable                            | none                     | each use raises the level by one, up to `trace`                                                           | SM-D15                                                               |
| `--quiet`         | `-q`  | none; repeatable                            | none                     | each use lowers the level by one, down to `error`                                                         | SM-D15                                                               |
| `--help`          | `-h`  | none                                        | none                     | prints usage and exits with status 0                                                                      | SM-D15                                                               |
| `--version`       | `-V`  | none                                        | none                     | prints the version and exits with status 0                                                                | SM-D15                                                               |

#### TX Options

These are the options of `tx`. `tx` accepts `-` or a file as its input; SM-D16 gives the spelling below.

| Long            | Short | Value                   | Default                                               | Constraints                                                              | Trace                                                      |
| --------------- | ----- | ----------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------ | ---------------------------------------------------------- |
| `--listen`      | `-l`  | `addr:port`             | `127.0.0.1:31100`                                     | loopback address only; port 1 to 65535; used only when `INPUT` is absent | one TCP connection in on localhost; default SM-D12; SM-D16 |
| `--remote`      | `-r`  | host name or IP address | `127.0.0.1`                                           | resolves to one address; how the two sides agree on it is a gap          | SM-D15; gap                                                |
| `INPUT` operand | none  | `-` or a file path      | absent: `tx` accepts one TCP connection at `--listen` | `-` reads standard input; any other value names a file                   | accepts `-` or a file; spelling SM-D16; gap                |

#### TX-PAT Options

These are the options of `tx-pat`. The seed is random by default, and an option sets a fixed seed; SM-D17 gives the spelling below.

| Long       | Short | Value                   | Default     | Constraints                                                | Trace                                                                          |
| ---------- | ----- | ----------------------- | ----------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `--remote` | `-r`  | host name or IP address | `127.0.0.1` | as for `tx`                                                | SM-D15; gap                                                                    |
| `--seed`   | `-s`  | unsigned 64-bit integer | random      | absent: `tx-pat` picks a random seed and logs it at `info` | random by default or a fixed seed set by an option; spelling SM-D17; gap       |

#### RX Options

| Long        | Short | Value       | Default           | Constraints                                                           | Trace          |
| ----------- | ----- | ----------- | ----------------- | --------------------------------------------------------------------- | -------------- |
| `--bind`    | `-b`  | IP address  | `127.0.0.1`       | a local address; `0.0.0.0` or `::` accepts datagrams from other hosts | SM-D11, SM-D15 |
| `--connect` | `-c`  | `addr:port` | `127.0.0.1:31200` | port 1 to 65535                                                       | SM-D14         |

#### Option Precedence

An option given on the command line overrides its default. A repeated value option takes its last value, including a global option given both before and after the subcommand; `-v` and `-q` count from both positions. `--log-level` sets the starting level, then each `-v` raises it and each `-q` lowers it by one, in any order. `--help` and `--version` take precedence over every other option and over a missing subcommand. Other configuration sources, such as a file or environment variables, are a gap.

#### Invalid Combinations

Each of these is a usage error with exit status 2 and no socket opened.

- No subcommand, or a subcommand other than `tx`, `tx-pat` or `rx`.
- A command option before the subcommand.
- An option of one subcommand with another subcommand; `--remote` belongs to both `tx` and `tx-pat`.
- More than one `INPUT` operand, or `--listen` together with an `INPUT` operand.
- `--streams` below 1, a port outside 1 to 65535, or base port + N - 1 above 65535.
- `--listen` with a non-loopback address.
- `--datagram-size` out of range, or `--arena-size` that gives W below 1 or at least half the sequence space, 2^(S-1).
- A missing, malformed or non-numeric option value.

#### Exit Status

PROPOSAL (SM-D15):

| Status | Meaning                                                                                                                                                                                                                                                                                                                                                       |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0      | The stream ended normally: `tx` after its input ended (its TCP client closed, or standard input or the file reached end of file) and the end-of-stream datagrams were sent, RX after it delivered the stream through end of stream, including a stream that timeout rule B ended (SM-D19); also `--help` and `--version`. Loss gaps do not change the status. |
| 1      | Runtime failure: a bind, connect, resolve, file open or read, or socket I/O error, or a fatal sequence ambiguity.                                                                                                                                                                                                                                             |
| 2      | Usage error, as listed under Invalid Combinations.                                                                                                                                                                                                                                                                                                            |

#### Usage Examples

Back-to-back verification loop on one host, with every default (four terminals; `nc` stands for any TCP client and sink):

```sh
nc -l 127.0.0.1 31200 > received.bin    # sink for the reconstituted stream
socket-mux rx                           # binds UDP 127.0.0.1:31000, connects to 127.0.0.1:31200
socket-mux tx                           # listens on 127.0.0.1:31100, sends to 127.0.0.1:31000
nc 127.0.0.1 31100 < original.bin       # source stream
cmp original.bin received.bin           # equal when no loss gap was logged
```

The same loop with `tx` reading a file, then standard input, instead of a TCP client:

```sh
socket-mux tx original.bin              # reads the file
cat original.bin | socket-mux tx -      # reads standard input
```

A pattern loop with a fixed seed, so the validator can regenerate the pattern; the validator is a gap:

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

PROPOSAL (SM-D10). The header that every datagram carries is current (SM-D2); this layout is not approved. Multi-byte fields use network byte order (big-endian). The sequence field is S/8 bytes, so the header is 14 + S/8 bytes, 18 in a normal 32-bit build, followed by `payload length` bytes of slice data. The header carries no session identifier.

| Field          | Offset       | Size (bytes) | Byte order | Purpose                                                                                                               |
| -------------- | ------------ | ------------ | ---------- | --------------------------------------------------------------------------------------------------------------------- |
| magic          | 0            | 2            | network    | Constant `0x534D` ("SM"); marks a socket-mux datagram.                                                                |
| version        | 2            | 1            | n/a        | Header format version; this layout is version 1.                                                                      |
| flags          | 3            | 1            | n/a        | Bit 0 = end of stream; bits 1 to 7 are zero.                                                                          |
| payload length | 4            | 2            | network    | Number of slice-data bytes after the header.                                                                          |
| sequence       | 6            | S/8 (4)      | network    | Global slice sequence number across all N lanes: 0 for the first slice, one more modulo 2^S for each following slice. |
| stream offset  | 6 + S/8 (10) | 8            | network    | Byte offset of this slice's first byte in the original stream.                                                        |

## Invariants

- N must be at least 1.
- Each lane must use a different port.
- Every UDP datagram must carry the datagram header.
- Neither mode retransmits a lost datagram.
- `tx` reads exactly one input: one TCP connection on localhost, standard input (`-`) or one file.
- `tx-pat` reads no input stream; it generates the pattern continuously and sends it under the same port and header rules as `tx`.
- `tx-pat` uses a random seed unless an option sets a fixed seed.
- RX's output from a pattern loop must be checkable against the pattern from any point after a loss gap, under any pattern of UDP datagram loss, including adversarial loss.
- RX delivers strictly in sequence order and never delivers a datagram out of sequence (SM-D19).
- No datagram waits behind a gap longer than the timeout T. A datagram released in sequence order waits only for the TCP output; the arena, not T, bounds those ([loss limits](#semantics)).
- Neither mode busy-waits or polls anywhere. All I/O is event-driven, so RX holding datagrams or waiting for them does not keep a CPU busy (SM-D20).

## Semantics

- Ports (SM-D11): lane k, for k from 0 to N - 1, uses base port + k on both sides.
- Loss: RX determines packet loss and the length of that loss from the datagram headers. PROPOSAL (SM-D13): the sequence gap gives the lost slice count and the stream-offset gap gives the lost byte length; RX skips the lost bytes, keeps the order of the bytes it delivers, and logs the gap (SM-P09). RX declares a missing sequence number lost only under timeout rule A or B or the arena limit below.
- Input: `tx` splinters its input across the N UDP lanes, and RX reassembles it on the remote end, so `tx` and RX act like a netcat split over N UDP lanes. `tx` accepts `-` (standard input) or a file as its input.
- Pattern: `tx-pat` generates a continuous pseudorandom Adler pattern, from a random seed by default or a fixed seed set by an option, instead of accepting an input stream to split. The pattern's format, where Adler checksums appear in it and whether `tx-pat` ends are gaps (OD-19, OD-21).
- Pattern verification: `tx-pat` paired with RX is a verification loop. RX's reconstituted output is validated against the known pattern of a fixed seed, and validation recovers after any UDP loss, including adversarial loss. How the pattern allows resynchronization (OD-19) and which program validates it (OD-20) are gaps.
- End of stream (SM-D19): the end-of-stream datagram carries a sequence number and a payload of zero or more bytes. RX processes it only in sequence order, or when the timeout releases it under rule A or rule B below. Rule B therefore also ends a stream whose end-of-stream datagram was lost. PROPOSAL (SM-D10): `tx` marks its final datagram with the end-of-stream flag after its input ends, and RX closes its TCP output after delivering through that datagram.
- Timeout (SM-D19): RX has one timeout, T, and applies two rules with it. Each arrival below is an arrival time ([Terminology](#terminology)).
  - Rule A: a run of missing sequence numbers is declared lost once T has passed since the arrival of the datagram just before the run, which is the received datagram with the highest sequence number below the run. Every sequence number in the run expires with it, and RX delivers on in sequence order from the next datagram it holds.
  - Rule B: once T has passed since the last datagram RX received, with no new datagram, RX declares every remaining gap lost, delivers everything it holds in sequence order, and ends the stream.
  - Worked example: datagrams 1, 2, 6 and 7 arrive at 0 s, and the stream continues to 10005 and 10006 at 10 s. With T = 10 s, at 10 s the inter-packet delay from 2 to 3 has expired, so 3 is lost, and 4 and 5 expire with it.
- Start of stream: RX waits without a limit for its first datagram and starts at it. Sequence numbers older than the first datagram are a missing run under rule A, with the first datagram's arrival in place of the datagram before the run: one that arrives within T of it is delivered in sequence order, and one that arrives later is late. How far before the first datagram that run extends, and whether its expiry is logged as a loss gap, is a gap (OD-23).
- Arena limit: a datagram whose sequence number is at least next + W has no place in the arena. RX declares the run at next lost, and the run after it as needed, until the datagram fits.
- Drain limit: when no arena buffer is free, because every buffer holds a datagram waiting in sequence order or for its TCP write, RX drops the newest datagram and counts it as lost.
- Late and duplicate datagrams: a datagram whose sequence number RX has already delivered or declared lost, or whose sequence number RX already holds, is dropped and logged.
- Loss limits: T, the arena and the TCP output are three independent limits on what RX holds. T is the slow limit: rules A and B bound how long a datagram waits behind a gap. The arena is the fast limit: the arena limit bounds how far ahead of a gap RX holds datagrams. The TCP output is the drain: RX keeps each datagram's buffer until its TCP write completes, so a slow TCP consumer fills the arena and the drain limit drops. T covers slow arrival and the arena covers fast arrival, so each is set without changing the other.
- Backpressure: RX applies no backpressure to TX.
- Sequence numbers: they always roll over, modulo 2^S, and RX compares them with serial-number arithmetic: s is after next when (s - next) mod 2^S is below 2^(S-1), and before next when it is above. RX tolerates no sequence ambiguity: a gap of half the sequence space, 2^(S-1), is a fatal error, and RX logs it and exits with status 1 (PROPOSAL). W must therefore be below 2^(S-1).
- Verification loop: TX and RX can be placed back to back in a verification loop, with either `tx` or `tx-pat` as TX.

## Diagnostics

- Loss gap (SM-P09): trigger: RX skips a loss gap; facts: the cause, `timeout` (rule A or B), `arena` (arena limit) or `drain` (drain limit); PROPOSAL: first missing sequence number, lost slice count, lost byte length, running totals.
- Late or duplicate datagram: trigger: RX drops one; facts: its sequence number and whether it was late or a duplicate; PROPOSAL: at `debug`.
- Timeout end of stream: PROPOSAL (SM-D21); trigger: timeout rule B ends the stream; facts: the timeout T and the running loss totals.
- Sequence ambiguity: trigger: a gap of half the sequence space; facts: next and the received sequence number; PROPOSAL: at `error`, then exit status 1.
- Exit status: PROPOSAL; see [Exit Status](#exit-status).

## Boundaries

- Platforms: socket-mux runs on Linux, macOS and Windows.
- Retransmission is out of scope at this stage.
- A stalled TCP output fills the arena (drain limit). Slice assignment to lanes (OD-4), any timeout other than T (OD-5), encryption and authentication (OD-6), whether TX slows its input when its own sends cannot keep up (OD-7), configuration sources other than the command line (OD-9), far-end addressing agreement (OD-10), build and test targets (OD-11), `tx` input selection (OD-17), the seed option (OD-18), the pattern format and resynchronization (OD-19), the pattern validator (OD-20), the end of `tx-pat` (OD-21) and the extent of the start-of-stream run (OD-23) are gaps.
