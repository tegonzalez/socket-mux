#!/bin/bash
# Link characterisation sweep: lanes x RX udp buffer x TX inter-packet delay.
# One key=value line per run in $OUT/runs.kv; summarised to CSV afterwards.
set -u
TS=$(date -u +%Y%m%dT%H%M%SZ); OUT=/tmp/smq/sweep-$TS; mkdir -p "$OUT"
RD="\$HOME/.local/state/socket-mux/sweeps/$TS"
BIN=/tmp/smq/socket-mux-new; BYTES=3000000000; REPS=${REPS:-2}
LANES=${LANES:-"1 2 4 8"}; BUFS=${BUFS:-"4194304 16777216 67108864"}; IPDS=${IPDS:-"0 1250 1350 1500 2000 3000"}; DGSS=${DGSS:-1452}
qd() { tc -s qdisc show dev enp191s0 | awk '/dropped/{gsub(",","",$7);print $7;exit}'; }
{ echo "sweep=$TS dir=spike->chuck bytes=$BYTES datagram_sizes=${DGSS// /,} mtu=$(ip -o link show enp191s0 | grep -o "mtu [0-9]*" | cut -d" " -f2) rx_memory=67108864 rx_timeout=2 tx_udp_buffer=default"
  echo "binary_md5=$(md5sum $BIN | cut -d' ' -f1) mtu=1500 link=10GbE rmem_max=67108864"; } > "$OUT/meta.txt"
for rep in $(seq $REPS); do for N in $LANES; do for B in $BUFS; do for I in $IPDS; do for G in $DGSS; do
  D="$RD/r$rep-n$N-b$B-i$I-g$G"
  ssh chuck "\$HOME/.local/state/socket-mux/runs/sweep-rx.sh start $D $N $B --timeout 2 --datagram-size $G" </dev/null
  q0=$(qd); t0=$(date +%s.%N)
  timeout 60 $BIN -n $N --datagram-size $G tx-pat --remote 192.168.7.40 --seed 42 --bytes $BYTES --ipd-ns $I >/dev/null 2>&1; tx=$?
  t1=$(date +%s.%N); q1=$(qd)
  rx=$(ssh chuck "\$HOME/.local/state/socket-mux/runs/sweep-rx.sh finish $D" </dev/null)
  echo "ts=$(date -u +%FT%TZ) rep=$rep lanes=$N rx_udp_buffer=$B ipd_ns=$I datagram_size=$G mtu=$(ip -o link show enp191s0 | grep -o "mtu [0-9]*" | cut -d" " -f2) tx_exit=$tx tx_secs=$(echo "$t1-$t0"|bc) tx_qdisc_drops=$((q1-q0)) $rx" | tee -a "$OUT/runs.kv"
done; done; done; done; done
echo "DONE $OUT"
