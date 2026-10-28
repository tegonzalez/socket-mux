---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-spec]; otherwise, do not modify."
---

# socket-mux Specification

## Purpose

Define the consumer-visible behavior of socket-mux: its TX and RX modes and four subcommands, its command-line interface, the input TX accepts or generates, the UDP lanes and per-datagram header it uses, the stream it reconstitutes, its configurable lane count and the platforms it supports. This specification defines the ratified intended baseline for the socket-mux CLI, including its TX and RX modes and the `check-pat` subcommand; it does not assert that every specified behavior is implemented in any particular source revision. "Must" marks a requirement. "Gap" marks behavior this specification does not define; and a consumer relies on no particular behavior there.

## Terminology

| Term              | Meaning                                                                                                                                                                                                                            |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| TX                | Transmit mode, run by the `tx` or the `tx-pat` subcommand. It splinters its input into N UDP lanes.                                                                                                                                |
| `tx` input        | The input of the `tx` subcommand: one TCP connection on localhost, standard input (`-`) or a file.                                                                                                                                 |
| Pattern           | The continuous pseudorandom pattern that `tx-pat` generates as its input, also called an "adler pattern".                                                                                                                          |
| Seed              | The value from which `tx-pat` generates the pattern: random by default, or a fixed seed set by an option.                                                                                                                          |
| RX                | Receive mode. It receives the N UDP lanes and reconstitutes the original stream.                                                                                                                                                   |
| N                 | The number of UDP lanes, which is also the number of slices.                                                                                                                                                                       |
| Lane              | One of the N UDP datagram flows from TX to RX; each uses a different port.                                                                                                                                                         |
| Slice             | A part of the original stream carried in one UDP datagram; the header sequences slices on reconstitution.                                                                                                                          |
| Base port         | The first port of the N lane ports; lane k (0 to N-1) uses base port + k.                                                                                                                                                          |
| Session           | One TX run, identified by a random 16-bit session ID chosen at start and carried in every datagram header. RX accepts one session per process.                                                                                     |
| Datagram header   | The header every UDP datagram carries.                                                                                                                                                                                             |
| Timeout T         | RX's one timeout, set by a global command-line option, default 10 s; spelled `--timeout` ([Inter-Packet Delay Timeout Spelling](design.md#inter-packet-delay-timeout-spelling)). It is also called the inter-packet delay timeout. |
| Loss gap          | A run of consecutive slices RX does not receive.                                                                                                                                                                                   |
| Datagram size     | The size of every UDP datagram, header included, set by a global option.                                                                                                                                                           |
| RX memory limit   | The maximum receive memory RX may use for datagrams it holds out of order or has not yet written to its TCP output; set by a global option.                                                                                        |
| Hold capacity W   | The number of datagrams that fit in the RX memory limit: RX memory limit ÷ datagram size, rounded down.                                                                                                                            |
| Sequence width S  | The effective bit width of the sequence counter, a build constant: 64 in normal builds, 8 or 16 in builds that test rollover. The wire field remains u64 at every width.                                                           |
| Next              | The lowest sequence number RX has neither delivered nor declared lost.                                                                                                                                                             |
| Arrival time      | The time RX's application dequeues a datagram from its socket, read from a monotonic clock; not a kernel timestamp.                                                                                                                |
| Verification loop | TX and RX placed back to back; a pattern loop is `tx-pat` and RX placed back to back.                                                                                                                                              |

## Model

TX reads one input and splinters it into N UDP lanes, each on a different port, with a header on every datagram. The `tx` subcommand reads one TCP connection on localhost, standard input (`-`) or a file, like netcat, and RX reassembles that stream on the remote end. The `tx-pat` subcommand behaves as `tx` does, except that its input is a pattern it generates continuously instead of an input stream. The `check-pat` subcommand reads a pattern stream from standard input and validates it; it opens no socket. RX uses the headers to sequence slices and to determine packet loss and the length of that loss, and reconstitutes the original stream. Neither network mode retransmits, so the channel is lossy.

```text
TCP client (127.0.0.1:31100), standard input or file -> tx -> UDP base port .. base port + N - 1 -> RX -> TCP peer (127.0.0.1:31200)
generated pattern                                      -> tx-pat -> UDP base port .. base port + N - 1 -> RX -> TCP peer -> pattern validation
```

The ports in the diagram are the defaults ([Default Ports](design.md#default-ports)).

## Public Surface

| ID     | Kind                       | Visibility | Use mode                                  | Exact shape                                                                                                                                                                                                                         | Trigger                                         | Error condition                                                                                                          | Derivation                                                                                                                                                                                                    |
| ------ | -------------------------- | ---------- | ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SM-P01 | CLI subcommand             | public     | invoked by operator                       | `socket-mux [GLOBAL OPTIONS] tx [TX OPTIONS]`; options in [Command Line](#command-line)                                                                                                                                             | Operator starts transmit mode                   | Usage error, exit status 2                                                                                               | design [One Program With TX And RX Subcommands](design.md#one-program-with-tx-and-rx-subcommands), [Command-Line Options](design.md#command-line-options)                                                     |
| SM-P10 | CLI subcommand             | public     | invoked by operator                       | `tx-pat`: TX whose input is the pattern instead of an input stream; `socket-mux [GLOBAL OPTIONS] tx-pat [TX-PAT OPTIONS]`; options in [Command Line](#command-line)                                                                 | Operator starts pattern transmit                | Usage error, exit status 2                                                                                               | design [One Program With TX And RX Subcommands](design.md#one-program-with-tx-and-rx-subcommands), [Command-Line Options](design.md#command-line-options), [TX-PAT Seed Option](design.md#tx-pat-seed-option) |
| SM-P02 | CLI subcommand             | public     | invoked by operator                       | `socket-mux [GLOBAL OPTIONS] rx [RX OPTIONS]`; options in [Command Line](#command-line)                                                                                                                                             | Operator starts receive mode                    | Usage error, exit status 2                                                                                               | design [One Program With TX And RX Subcommands](design.md#one-program-with-tx-and-rx-subcommands), [Command-Line Options](design.md#command-line-options)                                                     |
| SM-P03 | TCP ingress                | public     | connected to by a local client            | One TCP connection on localhost, one of the three `tx` inputs, accepted by `tx` at `--listen`; default `127.0.0.1:31100`                                                                                                            | A local client connects to `tx`                 | Bind failure: exit status 1                                                                                              | design [Default Ports](design.md#default-ports)                                                                                                                                                               |
| SM-P11 | Stream ingress             | public     | read by `tx`                              | `-` reads standard input; a file path reads that file; either is a `tx` input in place of the TCP connection. An optional `INPUT` operand selects TCP, `-` or a file.                                                               | `tx` starts with `-` or a file                  | Open or read failure: exit status 1                                                                                      | design [One Program With TX And RX Subcommands](design.md#one-program-with-tx-and-rx-subcommands), [TX Input Operand](design.md#tx-input-operand)                                                             |
| SM-P04 | UDP lane set               | public     | sent by TX, received by RX                | N UDP lanes on ports base port to base port + N - 1; sequence s uses lane s mod N, and TX sends lane k to `--remote` at base port + k; default N = 1; default base port 31000                                                       | TX carries data from its input                  | A datagram that does not arrive is declared lost under timeout rule A or B or the memory limit ([Semantics](#semantics)) | design [TX Splinters Over N Ports](design.md#tx-splinters-over-n-ports), [RX Listen Ports](design.md#rx-listen-ports), [Default Ports](design.md#default-ports)                                               |
| SM-P05 | Datagram header            | public     | carried on every UDP datagram             | A header per UDP datagram that lets RX sequence slices on reconstitution and determine packet loss and the length of that loss; layout in [Datagram Header](#datagram-header); each datagram is at most the datagram size (SM-P14)  | TX sends a datagram                             | RX discards invalid datagrams; session changes end the current stream under [Semantics](#semantics)                      | design [Header Per UDP Datagram](design.md#header-per-udp-datagram), [Datagram Header Layout](design.md#datagram-header-layout)                                                                               |
| SM-P06 | RX listen                  | public     | bound by RX                               | RX binds base port to base port + N - 1 (lane k on base port + k); address option `--bind` ([Command-Line Options](design.md#command-line-options))                                                                                 | RX starts                                       | Bind failure: exit status 1                                                                                              | design [RX Reconstitutes The Stream](design.md#rx-reconstitutes-the-stream), [RX Listen Ports](design.md#rx-listen-ports)                                                                                     |
| SM-P07 | Reconstituted stream       | public     | produced by RX                            | The original stream, reconstituted from the N UDP lanes; RX connects out over TCP to `--connect`, default `127.0.0.1:31200`, and writes the stream in order, skipping loss gaps                                                     | RX receives datagrams                           | Connect failure: exit status 1                                                                                           | design [RX Reconstitutes The Stream](design.md#rx-reconstitutes-the-stream), [Output Under Loss](design.md#output-under-loss), [RX Delivery Over TCP](design.md#rx-delivery-over-tcp)                         |
| SM-P08 | Setting: lane count        | public     | configured by operator                    | N is configurable with a minimum of 1, and its default is configurable; default 1; spelling: `--lanes`                                                                                                                              | Either mode starts                              | N below 1: usage error, exit status 2                                                                                    | design [Command-Line Options](design.md#command-line-options)                                                                                                                                                 |
| SM-P09 | Diagnostic: loss gap       | public     | logged by RX                              | One log line per loss gap naming its cause; at `warn`, also naming the lost slice count and byte length, plus running totals                                                                                                        | RX skips a loss gap                             | None                                                                                                                     | design [Output Under Loss](design.md#output-under-loss)                                                                                                                                                       |
| SM-P12 | Setting: pattern seed      | public     | configured by operator                    | `tx-pat` generates the pattern from a random seed by default; `--seed U64` sets a fixed seed                                                                                                                                        | `tx-pat` starts                                 | Invalid seed: usage error, exit status 2                                                                                 | design [TX-PAT Seed Option](design.md#tx-pat-seed-option)                                                                                                                                                     |
| SM-P13 | Pattern                    | public     | generated by `tx-pat`, validated after RX | A continuous sequence of self-locating 4096-byte blocks; format is specified under [Semantics](#semantics) and checking under [Validation](#validation)                                                                             | `tx-pat` runs                                   | None                                                                                                                     | design [Self-Locating Adler Pattern Blocks](design.md#self-locating-adler-pattern-blocks)                                                                                                                     |
| SM-P14 | Setting: datagram size     | public     | configured by operator                    | The UDP datagram size, header included, is configurable; `--datagram-size`, default 1452                                                                                                                                            | Either mode starts                              | Out of range: usage error, exit status 2                                                                                 | design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo), [Command-Line Options](design.md#command-line-options)                                                                    |
| SM-P15 | Setting: RX memory limit   | public     | configured by operator                    | RX's configured memory limit for held datagrams; W = RX memory limit ÷ datagram size. `--rx-memory`, default 64 MiB                                                                                                                 | RX starts                                       | Out of range: usage error, exit status 2                                                                                 | design [RX Memory Limit, Wheel And FIFO](design.md#rx-memory-limit-wheel-and-fifo), [Command-Line Options](design.md#command-line-options)                                                                    |
| SM-P16 | CLI subcommand             | public     | invoked by operator                       | `socket-mux [GLOBAL OPTIONS] check-pat [GLOBAL OPTIONS] [--seed U64]`; reads standard input, takes no operand and opens no socket; reports valid/bad blocks, resynchronization, losses, skipped/trailing bytes and accepted indices | Operator validates pattern output               | Usage error: exit status 2; read failure or invalid pattern: exit status 1                                               | design [Check-Pat Checker Subcommand](design.md#check-pat-checker-subcommand)                                                                                                                                 |
| S-17   | Diagnostic: TX session     | public     | logged by TX                              | `session=<n>` at `info`, naming the session ID                                                                                                                                                                                      | `tx` or `tx-pat` starts                         | None                                                                                                                     | design [Session Identity And Restart](design.md#session-identity-and-restart)                                                                                                                                 |
| S-18   | Diagnostic: new RX session | public     | logged by RX                              | A `warn` line reports that a new session arrived; RX finishes the current stream and exits 0                                                                                                                                        | A valid datagram carries a different session ID | None                                                                                                                     | design [Session Identity And Restart](design.md#session-identity-and-restart)                                                                                                                                 |

### Command Line

#### Grammar

The binary is `socket-mux` with this grammar ([Command-Line Options](design.md#command-line-options)). socket-mux is a CLI with TX and RX subcommands and a `tx-pat` subcommand ([One Program With TX And RX Subcommands](design.md#one-program-with-tx-and-rx-subcommands)). `tx` takes an optional `INPUT` operand ([TX Input Operand](design.md#tx-input-operand)).

```text
socket-mux [GLOBAL OPTIONS] tx [GLOBAL OPTIONS | TX OPTIONS]... [INPUT]
socket-mux [GLOBAL OPTIONS] tx-pat [GLOBAL OPTIONS | TX-PAT OPTIONS]...
socket-mux [GLOBAL OPTIONS] rx [GLOBAL OPTIONS | RX OPTIONS]...
socket-mux [GLOBAL OPTIONS] check-pat [GLOBAL OPTIONS | CHECK-PAT OPTIONS]...
socket-mux --help | --version
```

Global options are accepted both before and after every subcommand, including `check-pat`. Command options appear after the subcommand ([Command-Line Options](design.md#command-line-options)). TX sends lane k to `--remote` at base port + k, and RX listens on those same ports. The operator configures both sides with the same lane count, base port and datagram size by hand; they do not negotiate.

#### Global Options

The Trace column names the source of each setting.

| Long              | Short | Value                                       | Default                  | Constraints                                                                                                                                                                                                                              | Trace                                                                                                                                                                                  |
| ----------------- | ----- | ------------------------------------------- | ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--lanes`         | `-n`  | integer N                                   | 1                        | N >= 1; base port + N - 1 <= 65535                                                                                                                                                                                                       | N configurable with a configurable default; default 1, minimum 1                                                                                                                       |
| `--base-port`     | `-p`  | integer port                                | 31000                    | 1 to 65535; base port + N - 1 <= 65535                                                                                                                                                                                                   | base port incremented by one; [TX Splinters Over N Ports](design.md#tx-splinters-over-n-ports), [RX Listen Ports](design.md#rx-listen-ports), [Default Ports](design.md#default-ports) |
| `--udp-buffer`    | none  | integer bytes                               | operating-system default | >= 1; TX sets each UDP send buffer, RX each UDP receive buffer                                                                                                                                                                           | [Command-Line Options](design.md#command-line-options)                                                                                                                                 |
| `--timeout`       | none  | seconds                                     | 10                       | > 0; the timeout T of timeout rules A and B ([Semantics](#semantics), [Inter-Packet Delay Timeout](design.md#inter-packet-delay-timeout)); spelling [Inter-Packet Delay Timeout Spelling](design.md#inter-packet-delay-timeout-spelling) | one global RX timeout T, a CLI option, default 10 s; spelling [Inter-Packet Delay Timeout Spelling](design.md#inter-packet-delay-timeout-spelling)                                     |
| `--datagram-size` | none  | integer bytes                               | 1452                     | header size + 1 to 65507; TX sends no larger datagram; RX discards a larger one                                                                                                                                                          | datagram size configurable (SM-P14); spelling and default [Command-Line Options](design.md#command-line-options)                                                                       |
| `--rx-memory`     | none  | integer bytes                               | 67108864 (64 MiB)        | 1 <= W < 2^(S-1); RX uses it; TX accepts it and does not use it                                                                                                                                                                          | RX memory limit configurable (SM-P15); spelling and default [Command-Line Options](design.md#command-line-options)                                                                     |
| `--log-level`     | none  | `error`, `warn`, `info`, `debug` or `trace` | `info`                   | one of the listed values                                                                                                                                                                                                                 | [Command-Line Options](design.md#command-line-options)                                                                                                                                 |
| `--verbose`       | `-v`  | none; repeatable                            | none                     | each use raises the level by one, up to `trace`                                                                                                                                                                                          | [Command-Line Options](design.md#command-line-options)                                                                                                                                 |
| `--quiet`         | `-q`  | none; repeatable                            | none                     | each use lowers the level by one, down to `error`                                                                                                                                                                                        | [Command-Line Options](design.md#command-line-options)                                                                                                                                 |
| `--help`          | `-h`  | none                                        | none                     | prints usage and exits with status 0                                                                                                                                                                                                     | [Command-Line Options](design.md#command-line-options)                                                                                                                                 |
| `--version`       | `-V`  | none                                        | none                     | prints the version and exits with status 0                                                                                                                                                                                               | [Command-Line Options](design.md#command-line-options)                                                                                                                                 |

#### TX Options

These are the options of `tx`. `tx` accepts `-` or a file as its input; [TX Input Operand](design.md#tx-input-operand) gives the spelling below.

| Long            | Short | Value                   | Default                                               | Constraints                                                                                            | Trace                                                                                                                                |
| --------------- | ----- | ----------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| `--listen`      | `-l`  | `addr:port`             | `127.0.0.1:31100`                                     | loopback address only; port 1 to 65535; used only when `INPUT` is absent                               | one TCP connection in on localhost; default [Default Ports](design.md#default-ports); [TX Input Operand](design.md#tx-input-operand) |
| `--remote`      | `-r`  | host name or IP address | `127.0.0.1`                                           | TX sends lane k to the resolved host at base port + k; operator configures RX's matching ports by hand | design [TX Splinters Over N Ports](design.md#tx-splinters-over-n-ports), [RX Listen Ports](design.md#rx-listen-ports)                |
| `INPUT` operand | none  | `-` or a file path      | absent: `tx` accepts one TCP connection at `--listen` | `-` reads standard input; any other value names a file                                                 | accepts `-` or a file; spelling [TX Input Operand](design.md#tx-input-operand)                                                       |

#### TX-PAT Options

These are the options of `tx-pat`. The seed is random by default, and an option sets a fixed seed; [TX-PAT Seed Option](design.md#tx-pat-seed-option) gives the spelling below.

| Long       | Short | Value                   | Default     | Constraints                                                | Trace                                                                                                           |
| ---------- | ----- | ----------------------- | ----------- | ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `--remote` | `-r`  | host name or IP address | `127.0.0.1` | as for `tx`                                                | [Command-Line Options](design.md#command-line-options); gap                                                     |
| `--seed`   | `-s`  | unsigned 64-bit integer | random      | absent: `tx-pat` picks a random seed and logs it at `info` | random by default or a fixed seed set by an option; spelling [TX-PAT Seed Option](design.md#tx-pat-seed-option) |

#### RX Options

| Long        | Short | Value       | Default           | Constraints                                                           | Trace                                                                                                |
| ----------- | ----- | ----------- | ----------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `--bind`    | `-b`  | IP address  | `127.0.0.1`       | a local address; `0.0.0.0` or `::` accepts datagrams from other hosts | [RX Listen Ports](design.md#rx-listen-ports), [Command-Line Options](design.md#command-line-options) |
| `--connect` | `-c`  | `addr:port` | `127.0.0.1:31200` | port 1 to 65535                                                       | [RX Delivery Over TCP](design.md#rx-delivery-over-tcp)                                               |

#### Check-Pat Options

`check-pat` reads standard input, takes no operand and opens no socket. With `--seed`, it also regenerates and checks each block body; without it, it checks Adler-32 and block-index order. It accepts the global options above and this command option:

| Long     | Short | Value                   | Default | Constraints                                 | Trace                                                                         |
| -------- | ----- | ----------------------- | ------- | ------------------------------------------- | ----------------------------------------------------------------------------- |
| `--seed` | `-s`  | unsigned 64-bit integer | none    | malformed value: usage error, exit status 2 | design [Check-Pat Checker Subcommand](design.md#check-pat-checker-subcommand) |

#### Option Precedence

An option given on the command line overrides its default. A repeated value option takes its last value, including a global option given both before and after the subcommand; `-v` and `-q` count from both positions. `--log-level` sets the starting level, then each `-v` raises it and each `-q` lowers it by one, in any order. `--help` and `--version` take precedence over every other option and over a missing subcommand. Settings are supplied only through command-line options; there is no configuration file or environment-variable configuration.

#### Invalid Combinations

Each of these is a usage error with exit status 2 and no socket opened.

- No subcommand, or a subcommand other than `tx`, `tx-pat`, `rx` or `check-pat`.
- A command option before the subcommand.
- An option of one subcommand with another subcommand; `--remote` belongs to both `tx` and `tx-pat`.
- More than one `INPUT` operand, or `--listen` together with an `INPUT` operand.
- `--lanes` below 1, a port outside 1 to 65535, or base port + N - 1 above 65535.
- `--listen` with a non-loopback address.
- `--datagram-size` out of range, or `--rx-memory` that gives W below 1 or at least half the sequence space, 2^(S-1).
- A missing, malformed or non-numeric option value.

#### Exit Status

| Status | Meaning                                                                                                                                                                                                                                                                                                                                                                 |
| ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0      | The stream ended normally: `tx` after its input ended and the end-of-stream datagrams were sent, or RX after it delivered through end of stream, including when timeout rule B or a different session ended RX; also `check-pat` for empty input or input with at least one valid block and no bad block, `--help` and `--version`. Loss gaps do not change the status. |
| 1      | Runtime failure: a bind, connect, resolve, file open or read, or socket I/O error; fatal sequence ambiguity; or `check-pat` finding a bad block or nonempty input with no valid block.                                                                                                                                                                                  |
| 2      | Usage error, as listed under Invalid Combinations.                                                                                                                                                                                                                                                                                                                      |

Signal termination is outside these normal exit statuses: `tx-pat` installs no signal handler and uses the operating-system default disposition; Ctrl-C (SIGINT) commonly appears to the shell as status 130.

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

A pattern loop with a fixed seed, followed by `check-pat` validating the saved output:

```sh
nc -l 127.0.0.1 31200 > received.bin
socket-mux rx
socket-mux tx-pat --seed 42
socket-mux check-pat --seed 42 < received.bin
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

## Invariants

- N must be at least 1.
- Each lane must use a different port.
- Every UDP datagram must carry the datagram header.
- Neither mode retransmits a lost datagram.
- `tx` reads exactly one input: one TCP connection on localhost, standard input (`-`) or one file.
- `tx-pat` reads no input stream; it generates the pattern continuously and sends it under the same port and header rules as `tx`.
- `tx-pat` uses a random seed unless an option sets a fixed seed.
- RX's output from a pattern loop must be checkable against the pattern from any point after a loss gap, under any pattern of UDP datagram loss, including adversarial loss.
- RX delivers strictly in sequence order and never delivers a datagram out of sequence ([Inter-Packet Delay Timeout](design.md#inter-packet-delay-timeout)).
- No datagram waits behind a gap longer than the timeout T. A datagram released in sequence order waits only for the TCP output; the RX memory limit, not T, bounds those ([loss limits](#semantics)).
- Neither mode busy-waits or polls anywhere. All I/O is event-driven, so RX holding datagrams or waiting for them does not keep a CPU busy ([Event-Driven I/O](design.md#event-driven-io)).

## Contracts

### Datagram Header

The header that every datagram carries ([Header Per UDP Datagram](design.md#header-per-udp-datagram)) has this layout ([Datagram Header Layout](design.md#datagram-header-layout)). It is always 24 bytes, followed by `payload_len` bytes of slice data. Multi-byte fields use network byte order (big-endian). The sequence wire field is u64 even in reduced-width rollover-test builds. Version remains 1 throughout pre-alpha wire changes; version bumps begin with formal releases. There is no per-datagram application checksum.

| Field         | Offset (bytes) | Size (bytes) | Byte order | Purpose                                                                                                               |
| ------------- | -------------- | ------------ | ---------- | --------------------------------------------------------------------------------------------------------------------- |
| magic         | 0..2           | 2            | network    | Constant `0x534D` ("SM"); marks a socket-mux datagram.                                                                |
| version       | 2              | 1            | n/a        | Header format version, 1.                                                                                             |
| flags         | 3              | 1            | n/a        | Bit 0 = end of stream; bits 1 to 7 are zero.                                                                          |
| payload_len   | 4..6           | 2            | network    | Number of slice-data bytes after the header.                                                                          |
| session       | 6..8           | 2            | network    | Random 16-bit ID for this TX run.                                                                                     |
| sequence      | 8..16          | 8            | network    | Global slice sequence number across all N lanes: 0 for the first slice, one more modulo 2^S for each following slice. |
| stream_offset | 16..24         | 8            | network    | Byte offset of this slice's first byte in the original stream, modulo 2^64; it rolls over like the sequence number.   |

The default datagram size of 1452 bytes includes this header, leaving 1452 - 24 = 1428 payload bytes. At the default 64 MiB RX memory limit, W = floor(67108864 / 1452) = 46218 datagrams.


## Semantics

- Session ([Session Identity And Restart](design.md#session-identity-and-restart)): TX (`tx` and `tx-pat`) chooses a random 16-bit session ID at start, logs `session=<n>` at `info`, and carries that ID in every datagram, including end of stream. RX locks onto the session of its first valid datagram. A valid datagram from a different session triggers a `warn` log and ends the current stream exactly as end of stream does: RX delivers what the ordering and loss rules allow in order, closes its TCP output and exits 0. It does not deliver the different session's payload. A new stream requires a fresh RX process; there is no resume. A repeated session ID cannot distinguish runs, and the first valid datagram may belong to a stale session ([known limits](known-issues.md#register)).
- Ports ([RX Listen Ports](design.md#rx-listen-ports)): lane k, for k from 0 to N - 1, uses base port + k on both sides. TX sends lane k to `--remote` at base port + k; RX listens on the same ports. The operator supplies both sides the same lane count, base port and datagram size by hand; there is no negotiation.
- Loss: RX determines packet loss and the length of that loss from the datagram headers. The sequence gap gives the lost slice count and the stream-offset gap gives the lost byte length; RX skips the lost bytes, keeps the order of the bytes it delivers, and logs the gap (SM-P09). RX declares a missing sequence number lost only under timeout rule A or B or the memory limit below.
- Input: `tx` splinters its input across the N UDP lanes, and RX reassembles it on the remote end, so `tx` and RX act like a netcat split over N UDP lanes. `tx` accepts `-` (standard input) or a file as its input.
- Pattern: `tx-pat` generates an unpaced, continuous sequence of 4096-byte blocks and sends no end-of-stream datagram. Each block contains an 8-byte big-endian index starting at zero, 4084 bytes of SplitMix64 payload keyed by the seed and index, and a 4-byte big-endian Adler-32 checksum of the first 4092 bytes. The SplitMix64 state starts at `seed XOR (index × 0x9E3779B97F4A7C15)` with wrapping 64-bit multiplication; each output step adds `0x9E3779B97F4A7C15`, applies the SplitMix64 mixing operations, and emits the result big-endian, truncating the last output to four bytes. Adler-32 starts with A = 1 and B = 0, uses modulus 65521, and stores `(B << 16) | A`. `tx-pat` runs until a send error or external termination, installs no signal handler, and sends no end-of-stream marker. A signal uses the operating-system default disposition; Ctrl-C (SIGINT) terminates the process and commonly appears to the shell as status 130, not a normal successful exit.
- End of stream ([Inter-Packet Delay Timeout](design.md#inter-packet-delay-timeout)): the end-of-stream datagram carries a sequence number and a payload of zero or more bytes. RX processes it only in sequence order, or when the timeout releases it under rule A or rule B below. Rule B therefore also ends a stream whose end-of-stream datagram was lost. `tx` marks its final datagram with the end-of-stream flag after its input ends, and RX closes its TCP output after delivering through that datagram.
- Timeout ([Inter-Packet Delay Timeout](design.md#inter-packet-delay-timeout)): RX has one timeout, T, and applies two rules with it. Each arrival below is an arrival time ([Terminology](#terminology)).
  - Rule A: a run of missing sequence numbers is declared lost once T has passed since the arrival of the datagram just before the run, which is the received datagram with the highest sequence number below the run. Every sequence number in the run expires with it, and RX delivers on in sequence order from the next datagram it holds.
  - Rule B: once T has passed since the last datagram RX received, with no new datagram, RX declares every remaining gap lost, delivers everything it holds in sequence order, and ends the stream.
  - Worked example: datagrams 1, 2, 6 and 7 arrive at 0 s, and the stream continues to 10005 and 10006 at 10 s. With T = 10 s, at 10 s the inter-packet delay from 2 to 3 has expired, so 3 is lost, and 4 and 5 expire with it.
- Start of stream: RX waits without a limit for its first valid datagram, locks onto its session and anchors on it. Sequence ordering and timeout rules apply within that locked session. If that datagram's sequence number is s0, RX waits at most T from its arrival for sequence numbers older than s0: an older datagram that arrives within T is placed and delivered in sequence order. Once T has passed since that arrival, RX declares the older sequence numbers still missing lost and proceeds; it never waits longer, because a longer wait would break the timeout T. An older datagram that arrives later is late.
- Memory limit: a datagram whose sequence number is at least next + W has no place in RX's memory. RX declares the run at next lost, and the run after it as needed, until the datagram fits.
- Output limit: when no RX memory is free, because every held datagram waits in sequence order or for its TCP write, RX drops the newest datagram and counts it as lost.
- Late and duplicate datagrams: a datagram whose sequence number RX has already delivered or declared lost, or whose sequence number RX already holds, is dropped and logged.
- Loss limits: T, the RX memory limit and the TCP output are three independent limits on what RX holds. T is the slow limit: rules A and B bound how long a datagram waits behind a gap. The RX memory limit is the fast limit: it bounds how far ahead of a gap RX holds datagrams. The TCP output is the third limit: RX keeps each datagram's memory until its TCP write completes, so a slow TCP consumer fills the RX memory and the output limit drops. T covers slow arrival and the RX memory limit covers fast arrival, so each is set without changing the other.
- Backpressure: TX waits for each of its own UDP sends to complete, but RX sends no backpressure to TX. When the TCP consumer is slow, RX retains datagrams until its memory is full, then the output limit drops newest datagrams as loss; there is no separate TCP-output-stall timeout.
- Sequence numbers: they always roll over, modulo 2^S, and RX compares them with serial-number arithmetic: s is after next when (s - next) mod 2^S is below 2^(S-1), and before next when it is above. RX tolerates no sequence ambiguity: a gap of half the sequence space, 2^(S-1), is a fatal error, and RX logs it and exits with status 1. W must therefore be below 2^(S-1). The stream offset rolls over the same way, modulo 2^64, and RX computes the stream-offset gap of a loss modulo 2^64.
- Verification loop: TX and RX can be placed back to back in a verification loop, with either `tx` or `tx-pat` as TX.

## Validation

- Pattern verification: `check-pat` reads RX's output from standard input. It finds valid 4096-byte windows by Adler-32 and, when `--seed` is given, also regenerates and compares the payload. It accepts a block whose index follows the prior accepted index; otherwise it searches forward one byte at a time for the next valid window and reports resynchronization. Without a seed, it checks the checksum and index order but cannot verify the pseudorandom body.
- Checker report: one line is printed in the form `check-pat: blocks_ok=<n> blocks_bad=<n> resyncs=<n> lost_bytes=<n> skipped_bytes=<n> trailing_bytes=<n> first_index=<index|none> last_index=<index|none>`. It reports valid and bad blocks, resynchronizations, inferred missing bytes, bytes skipped while searching, bytes after the last complete valid block, and first/last accepted indices; absent indices are `none`.

## Diagnostics

- TX session (S-17): trigger: `tx` or `tx-pat` starts; facts: `session=<n>`, the chosen session ID; at `info`.
- New RX session (S-18): trigger: RX receives a valid datagram with a session ID different from its locked session; facts: a new session arrived; at `warn`, then the current stream ends under the session rule and RX exits 0.
- Loss gap (SM-P09): trigger: RX skips a loss gap; facts: the cause, `timeout` (rule A or B), `memory` (RX memory limit) or `output` (output limit); first missing sequence number, lost slice count, lost byte length, running totals.
- Late or duplicate datagram: trigger: RX drops one; facts: its sequence number and whether it was late or a duplicate; at `debug`.
- Timeout end of stream ([Inter-Packet Delay Timeout Spelling](design.md#inter-packet-delay-timeout-spelling)): trigger: timeout rule B ends the stream; facts: the timeout T and the running loss totals.
- Sequence ambiguity: trigger: a gap of half the sequence space; facts: next and the received sequence number; at `error`, then exit status 1.
- Exit status: see [Exit Status](#exit-status).

## Boundaries

- Platforms ([First-Release Platform Targets](design.md#first-release-platform-targets)): the first release (MVP) qualifies Linux only, on x86_64 and aarch64 with static musl builds. Each Linux target passes only after both a file transfer and a `tx-pat`/`check-pat` run pass on a real machine running Linux. A successful cross-build alone is not a pass. Windows 10/11 x86_64 and macOS are built, untested and unsupported for the first release.
- Security ([Trusted-Link Security Boundary](design.md#trusted-link-security-boundary)): socket-mux is for trusted or private links only. Lane traffic is neither encrypted nor authenticated: anyone on the path can read it, and anyone who can reach RX's UDP ports can inject datagrams. To cross an untrusted network, run socket-mux inside a VPN, for example WireGuard. Session IDs are not authentication.
- Retransmission is out of scope at this stage.
