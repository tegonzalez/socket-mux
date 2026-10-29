# link-sweep

Characterise one socket-mux UDP path as lanes x RX `--udp-buffer` x TX inter-packet delay (IPD).
Prototype harness; results stay on each host's local disk, never under `/mnt/shared`.

| File | Host | Role |
| --- | --- | --- |
| `sweep.sh` | TX (spike) | Drives the grid; one `key=value` line per run in `/tmp/smq/sweep-<ts>/runs.kv` |
| `sweep-rx.sh` | RX (chuck) | `start`/`finish` one RX run; snapshots UDP, NIC and softnet counters |
| `sink.py` | RX (chuck) | Discards RX output, reports active-window goodput |
| `summarize.py` | any | `runs.kv` -> `runs.csv` (per run) and `grid.csv` (per cell, over reps) |

Each run sends exactly `--bytes 3000000000` of `tx-pat` with `--ipd-ns I` and ends with EOS, so there is no
tail-kill artifact; RX uses `--timeout 2`. Override the grid with `LANES`, `BUFS`, `IPDS`, `REPS`.

Plot axes: x = `ipd_ns` (or `target_gbps`), y = `rx_udp_buffer_mib`, series/facet = `lanes`;
z = `goodput_mean`, `loss_ratio_max` or `lossless_runs/reps`. `nic_bound` and `nic_missed_max` separate
NIC ring overflow from socket-buffer (`rcvbuf_errors`) and RX (`lost_memory`, `lost_output`) loss.
