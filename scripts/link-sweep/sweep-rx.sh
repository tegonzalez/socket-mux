#!/bin/bash
# sweep-rx.sh start DIR N RXBUF [extra rx args] | finish DIR
# Host-local RX side of the link sweep; prints key=value fields.
# r8169 keeps rx_missed in a 16-bit counter, so nic_rx_missed is the delta mod 65536.
# SMBIN selects the binary. first_gap_seq needs rx at debug (pass -v as an extra rx arg).
R=$HOME/.local/state/socket-mux
snap() { # udp_in rcvbuf nic_rx_drop nic_rx_missed softnet_drop
  read -r _ in _ _ _ rb _ < <(grep -A1 '^Udp:' /proc/net/snmp | tail -1)
  read -r _ _ _ drop miss _ < <(ip -s link show enp191s0 | sed -n 4p)
  sd=0; while read -r _ d _; do sd=$((sd + 16#$d)); done < /proc/net/softnet_stat
  echo "$in $rb $drop $miss $sd"; }
case $1 in
start)
  D=$2; N=$3; BUF=$4; shift 4; mkdir -p "$D"; cd "$D"
  python3 $R/runs/sink.py > sink.txt 2>&1 & echo $! > sink.pid
  sleep 0.3; snap > snap.before
  ${SMBIN:-$R/bin/socket-mux} -n "$N" --udp-buffer "$BUF" "$@" rx --bind 192.168.7.40 > rx.log 2>&1 & echo $! > rx.pid
  sleep 0.3 ;;
finish)
  D=$2; cd "$D"
  for _ in $(seq 100); do kill -0 "$(cat rx.pid)" 2>/dev/null || break; sleep 0.1; done
  kill "$(cat rx.pid)" 2>/dev/null; sleep 0.3; kill "$(cat sink.pid)" 2>/dev/null
  snap > snap.after
  read -r i0 r0 d0 m0 s0 < snap.before; read -r i1 r1 d1 m1 s1 < snap.after
  sink=$(cat sink.txt)
  # rx summary: ... delivered B bytes in D datagrams  lost X datagrams Y bytes in K stretches (memory a output b timeout c) ...
  tot=$(grep -m1 'rx summary:' rx.log | sed -nE 's/.*delivered [0-9]+ bytes in ([0-9]+) datagrams +lost ([0-9]+) datagrams ([0-9]+) bytes in [0-9]+ stretches \(memory ([0-9]+) output ([0-9]+) timeout ([0-9]+)\).*/\1 \2 \3 \4 \5 \6/p' | awk '{printf "last_seq=%d total_lost_slices=%d total_lost_bytes=%d lost_memory=%d lost_output=%d lost_timeout=%d", $1+$2-1, $2, $3, $4, $5, $6}')
  fg=$(grep -m1 'loss gap:' rx.log | grep -o 'first_seq=[0-9]*' | cut -d= -f2)
  eff=$(grep -m1 'udp buffer requested' rx.log | grep -o 'effective=[0-9]*' | cut -d= -f2)
  echo "$sink udp_in=$((i1-i0)) rcvbuf_errors=$((r1-r0)) nic_rx_dropped=$((d1-d0)) nic_rx_missed=$(( (m1-m0+65536) % 65536 )) softnet_dropped=$((s1-s0)) rx_buf_effective=${eff:-NA} first_gap_seq=${fg:-NA} $tot" ;;
esac
