#!/usr/bin/env bash
# #141: after the other side resets a TCP connection, a read must still
# return the data that arrived before the reset; ENOTCONN only comes once
# the receive buffer is empty.
#
# In QEMU: scripts/tcp-reset-check.py plays a server on the guest's subnet
# over QEMU's socket netdev (the user network would turn the reset into a
# normal close). The guest connects with nc; the server sends one receive
# window of data (65535 bytes) and the reset right behind it. nc must write
# all of it before it reports the error.
#
# Without the fix nc gets ENOTCONN as soon as netstack has seen the reset
# and loses what it had not read by then (in QEMU usually all of it).
# Takes about 2 minutes.
#
# Usage:
#   scripts/test-tcp-reset-read.sh [qemu-raspi3b.sh options, e.g. --image <img>]
# TCP_RESET_SOCKET=<group:port> picks another multicast group (parallel runs).

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
sock=${TCP_RESET_SOCKET:-239.255.77.78:17778}
mkdir -p "$ROOT/images"
peer_log="$ROOT/images/tcp-reset-peer.log"
log="$ROOT/images/qemu-test.log"

python3 "$ROOT/scripts/tcp-reset-check.py" --socket "$sock" --seconds 600 \
    > "$peer_log" 2>&1 &
peer=$!

# nc writes "Remote host: <address>" to stdout before the data
"$ROOT/scripts/qemu-raspi3b.sh" --usb-net-socket "$sock" --timeout 500 "$@" \
    --run 'echo 10.0.3.15/24 > /scheme/netcfg/ifaces/eth0/addr/set' \
    --run 'nc 10.0.3.2:5000 < /scheme/null > /tmp/data ^> /tmp/nc-err; echo READ-BEGIN; wc -c < /tmp/data; head -c 12 /tmp/data; echo; echo READ-END; cat /tmp/nc-err' \
    < /dev/null > "$ROOT/images/tcp-reset-qemu.out" 2>&1
qemu_status=$?

wait "$peer"
peer_status=$?
cat "$peer_log"
sent=$(sed -n 's/^result: reset after \([0-9]*\) bytes$/\1/p' "$peer_log")
out=$(tr -d '\r' < "$log" | sed -n '/^READ-BEGIN$/,/^READ-END$/p')
read_bytes=$(echo "$out" | grep -E '^ *[0-9]+ *$' | tr -d ' ' | head -1)
if [ -n "$read_bytes" ] && echo "$out" | grep -q '^Remote host:$'; then
    read_bytes=$((read_bytes - $(printf 'Remote host: 10.0.3.2:5000\n' | wc -c)))
fi
echo "guest read: ${read_bytes:-nothing}; sent before the reset: ${sent:-no reset}"
tr -d '\r' < "$log" | sed -n '/^READ-END$/,$p' | grep -a -m1 'nc error' || true

if [ $qemu_status -ne 0 ] || [ $peer_status -ne 0 ] || [ -z "$sent" ] || [ -z "$read_bytes" ]; then
    echo "FAIL: test did not run through (QEMU exit $qemu_status, log $log)"
    exit 1
elif [ "$read_bytes" -eq "$sent" ]; then
    echo "PASS: the guest read every byte that arrived before the reset"
else
    echo "FAIL: the guest lost $((sent - read_bytes)) bytes that arrived before the reset"
    exit 1
fi
