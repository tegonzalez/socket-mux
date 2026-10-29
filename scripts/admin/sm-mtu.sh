#!/bin/sh
# sm-mtu.sh — show the MTU of the socket-mux test link and print the commands
# an administrator can run to switch it to jumbo frames (9000) and back.
#
# Read-only: changes nothing, needs no root, writes no files.
# Both hosts and every switch port between them must carry the same MTU.

DEV=enp191s0
JUMBO=9000

echo "# host: $(hostname)"
ip -d link show "$DEV" | grep -o 'mtu [0-9]*\|maxmtu [0-9]*' | sed 's/^/# /'
echo
echo "# jumbo on (run on both hosts):"
echo "sudo ip link set dev $DEV mtu $JUMBO"
echo
echo "# check the path end to end (no fragmentation allowed):"
echo "ping -c3 -M do -s $((JUMBO - 28)) <other host>"
echo
echo "# back to normal (run on both hosts; also undone by reboot):"
echo "sudo ip link set dev $DEV mtu 1500"
