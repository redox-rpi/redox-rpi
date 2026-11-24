#!/usr/bin/env bash
# #140: during a long download, netstack must not drop packets when the
# neighbour's ARP entry expires (once a minute).
#
# In QEMU: scripts/tcp-arp-check.py plays a server on the guest's subnet over
# QEMU's socket netdev that answers ARP only after 1.5 s (QEMU's user network
# answers at once, so the drop never shows there). The guest downloads for
# 150 s with nc, so its entry expires twice. Pass: the stream runs to the end
# with no pause of more than 0.5 s in the guest's ACKs.
#
# Without the fix every expiry holds the guest's ACKs back while it asks for
# the address again (and the queue of 10 overflows), so the stream stalls
# once a minute. Takes about 5 minutes.
#
# Usage:
#   scripts/test-tcp-arp.sh [qemu-raspi3b.sh options, e.g. --image <img>]
# TCP_ARP_SOCKET=<group:port> picks another multicast group (parallel runs).

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
sock=${TCP_ARP_SOCKET:-239.255.77.79:17779}
mkdir -p "$ROOT/images"
peer_log="$ROOT/images/tcp-arp-peer.log"
log="$ROOT/images/qemu-test.log"

python3 "$ROOT/scripts/tcp-arp-check.py" --socket "$sock" --seconds 150 --wait 700 \
    > "$peer_log" 2>&1 &
peer=$!

"$ROOT/scripts/qemu-raspi3b.sh" --usb-net-socket "$sock" --timeout 700 "$@" \
    --run 'echo 10.0.3.15/24 > /scheme/netcfg/ifaces/eth0/addr/set' \
    --run 'echo STREAM-BEGIN; nc 10.0.3.2:5000 < /scheme/null > /scheme/null; echo STREAM-END' \
    < /dev/null > "$ROOT/images/tcp-arp-qemu.out" 2>&1
qemu_status=$?

wait "$peer"
peer_status=$?
cat "$peer_log"
pauses=$(sed -n 's/^result: .*, \([0-9]*\) ACK pauses.*/\1/p' "$peer_log")

if [ $qemu_status -ne 0 ] || [ $peer_status -ne 0 ] || [ -z "$pauses" ]; then
    echo "FAIL: test did not run through (QEMU exit $qemu_status, peer exit $peer_status, log $log)"
    exit 1
elif [ "${pauses:-x}" = 0 ]; then
    echo "PASS: the stream never stalled while the ARP entry was renewed"
else
    echo "FAIL: the stream stalled ${pauses:-?} times (the guest held its ACKs back while it asked for the address)"
    exit 1
fi
