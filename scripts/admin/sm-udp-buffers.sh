#!/bin/sh
# sm-udp-buffers.sh — show the UDP buffer limits that affect socket-mux, and
# print the command an administrator can run to raise them.
#
# Read-only: changes nothing, needs no root, writes no files.

CAP=67108864   # 64 MiB

echo "# host: $(hostname)"
sysctl net.core.rmem_max net.core.wmem_max | sed 's/^/# /'
awk '/^Udp:/ { if (!h) { split($0, k); h = 1 } else for (i = 2; i <= NF; i++)
       if (k[i] == "RcvbufErrors") print "# udp RcvbufErrors since boot: " $i }' /proc/net/snmp
echo
echo "sudo sysctl -w net.core.rmem_max=$CAP net.core.wmem_max=$CAP"
