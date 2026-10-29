---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-constraints]; otherwise, do not modify."
---

# socket-mux Constraints

## Constraints

- **per-datagram-scan-proportional-to-window**: Per-datagram work in RX that scans the held wheel, the hold capacity W or the queued TCP output makes each datagram cost more exactly when RX is behind, so an output backlog grows into a memory- and output-limit loss cascade, which collapsed multi-lane goodput on 10GbE. Detected when RX CPU time per datagram, or loss at a fixed offered rate, rises with `--rx-memory` or with the output backlog instead of staying flat.
- **unbounded-hot-path-logging**: Formatting or writing a log line per lost stretch, per late or duplicate datagram, or per any other per-datagram event on a receiving, ordering, sending or output thread lets heavy loss flood the log and slow the hot path, which causes more loss. Detected when the number of log lines grows with loss or datagram events rather than with elapsed time, or when goodput falls as induced loss rises while log output rises with it.
- **idle-busy-wait-or-socket-polling**: Spinning without a bound, polling a socket with nonblocking calls or waking on a fixed interval to check for work keeps a CPU core busy, up to 100 %, while socket-mux is idle and adds latency when it is not. Detected when an idle TX or RX, or an RX holding datagrams behind a gap, accumulates measurable CPU time over several seconds, or when a socket receive or send returns would-block repeatedly in a loop.
- **large-tx-socket-buffer**: A large UDP send buffer on TX lets the sending host queue more datagrams than the link carries, so its queueing discipline drops them before the wire and RX sees loss that no receive setting removes. Detected when the TX host's qdisc or interface transmit drop counter rises during a run while RX shows loss without matching UDP receive-buffer errors.
