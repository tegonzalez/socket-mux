#!/usr/bin/env python3
"""Summarise a socket-mux link sweep (runs.kv) into CSV for 2-D/3-D plotting.

Usage: summarize.py SWEEP_DIR
Reads SWEEP_DIR/runs.kv (one key=value line per run, written by sweep.sh) and
writes:
  runs.csv  one row per run, raw counters plus derived columns
  grid.csv  one row per (lanes, rx_udp_buffer, ipd_ns): mean/min/max over reps
Derived columns:
  target_gbps      payload rate the IPD asks for: (datagram_size - 24) * 8 / ipd
  fragments        IP fragments per datagram at the run's MTU (grid.csv)
  goodput_gbps     delivered payload bits / active receive seconds
  loss_ratio       lost slices / (last_seq + 1)
  lossless         1 when no slice was lost
  early_loss       1 when a gap starts before the final 1 % of the stream
  nic_bound        1 when a lossy run also saw NIC rx missed/dropped datagrams
"""
import csv
import statistics
import sys
from collections import defaultdict
from pathlib import Path

PAYLOAD = 1428
NUM = {"datagram_size", "mtu", "rep", "lanes", "rx_udp_buffer", "ipd_ns", "tx_exit", "tx_secs", "tx_qdisc_drops",
       "bytes", "active_bytes", "active_secs", "Gbit/s", "udp_in", "rcvbuf_errors",
       "nic_rx_dropped", "nic_rx_missed", "softnet_dropped", "rx_buf_effective",
       "first_gap_seq", "last_seq", "total_lost_slices", "total_lost_bytes",
       "lost_memory", "lost_output", "lost_timeout"}


def parse(line):
    row = {}
    for field in line.split():
        if "=" in field:
            key, value = field.split("=", 1)
            if key in NUM:
                try:
                    value = float(value) if "." in value else int(value)
                except ValueError:
                    value = None
            row[key] = value
    return row


def derive(row):
    ipd = row.get("ipd_ns") or 0
    row.setdefault("datagram_size", PAYLOAD + 24)
    row.setdefault("mtu", 1500)
    payload = row["datagram_size"] - 24
    row["target_gbps"] = round(payload * 8 / ipd, 3) if ipd else ""
    row["goodput_gbps"] = row.pop("Gbit/s", None)
    last = row.get("last_seq")
    lost = row.get("total_lost_slices") or 0
    total = (last + 1) if isinstance(last, int) else None
    row["loss_ratio"] = (lost / total) if total else ""
    row["lossless"] = int(lost == 0)
    first = row.get("first_gap_seq")
    row["early_loss"] = int(isinstance(first, int) and total is not None and first < 0.99 * total)
    # One NIC miss can cost several slices (memory/output skips follow it), so
    # flag NIC involvement, not an exact count match.
    missed = (row.get("nic_rx_missed") or 0) + (row.get("nic_rx_dropped") or 0)
    row["nic_bound"] = int(lost > 0 and missed > 0)
    return row


def write_csv(path, fieldnames, rows):
    try:
        with open(path, "w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=fieldnames)
            w.writeheader()
            w.writerows(rows)
    except OSError as error:
        sys.exit(f"cannot write {path}: {error}")


def main():
    sweep = Path(sys.argv[1])
    try:
        lines = (sweep / "runs.kv").read_text().splitlines()
    except OSError as error:
        sys.exit(f"cannot read {sweep / 'runs.kv'}: {error}")
    rows = [derive(parse(line)) for line in lines if line.strip()]
    if not rows:
        sys.exit("no runs")
    keys = list(dict.fromkeys(k for r in rows for k in r))
    write_csv(sweep / "runs.csv", keys, rows)

    groups = defaultdict(list)
    for r in rows:
        groups[(r["mtu"], r["datagram_size"], r["lanes"], r["rx_udp_buffer"], r["ipd_ns"])].append(r)
    out = []
    for (mtu, dgs, lanes, buf, ipd), rs in sorted(groups.items()):
        good = [r["goodput_gbps"] for r in rs if isinstance(r["goodput_gbps"], (int, float))]
        loss = [r["loss_ratio"] for r in rs if r["loss_ratio"] != ""]
        out.append({
            "mtu": mtu, "datagram_size": dgs, "fragments": -(-(dgs + 8) // (mtu - 20)),
            "lanes": lanes, "rx_udp_buffer": buf, "rx_udp_buffer_mib": buf / 2**20, "ipd_ns": ipd,
            "target_gbps": rs[0]["target_gbps"], "reps": len(rs),
            "goodput_mean": round(statistics.mean(good), 3) if good else "",
            "goodput_min": min(good) if good else "", "goodput_max": max(good) if good else "",
            "loss_ratio_mean": statistics.mean(loss) if loss else "",
            "loss_ratio_max": max(loss) if loss else "",
            "lossless_runs": sum(r["lossless"] for r in rs),
            "early_loss_runs": sum(r["early_loss"] for r in rs),
            "rcvbuf_errors_max": max(r.get("rcvbuf_errors") or 0 for r in rs),
            "nic_missed_max": max(r.get("nic_rx_missed") or 0 for r in rs),
            "lost_memory_sum": sum(r.get("lost_memory") or 0 for r in rs),
            "lost_output_sum": sum(r.get("lost_output") or 0 for r in rs),
            "lost_timeout_sum": sum(r.get("lost_timeout") or 0 for r in rs),
        })
    write_csv(sweep / "grid.csv", list(out[0]), out)
    print(f"{len(rows)} runs -> {sweep/'runs.csv'}; {len(out)} cells -> {sweep/'grid.csv'}")


if __name__ == "__main__":
    main()
