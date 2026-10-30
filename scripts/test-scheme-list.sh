#!/usr/bin/env bash
# Boot an image in QEMU several times and count the boots in which netstack
# found no network.* scheme ("no network adapter found"). Meant for an image
# built with --experiment scheme-list-stress, which registers 16 schemes
# while netstack lists /scheme: without the experiment ns-list-snapshot
# many boots fail, with it none should.
#
# The host port for devshd is 12323, or DEVSH_PORT=<port>; BOOTS=<n>
# boots (default 10).
#
# Usage:
#   scripts/test-scheme-list.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
log="$ROOT/images/qemu-scheme-list.log"
boots=${BOOTS:-10}
failed=0

for i in $(seq "$boots"); do
    rm -f "$log"
    timeout 300 "$ROOT/scripts/qemu-raspi3b.sh" --usb-net --image "$image" \
        < /dev/null > "$log" 2>&1 &
    qemu=$!
    for _ in $(seq 250); do
        grep -aq "redox login:" "$log" && break
        sleep 1
    done
    sleep 3
    pkill -P $qemu 2>/dev/null
    kill $qemu 2>/dev/null
    wait $qemu 2>/dev/null
    if grep -aq "no network adapter found" "$log"; then
        failed=$((failed + 1))
        cp "$log" "$ROOT/images/qemu-scheme-list-fail-$i.log"
        echo "      boot $i: netstack found no network adapter"
    else
        echo "      boot $i: ok"
    fi
done

echo "$failed of $boots boots without a network adapter"
if [ "$failed" -eq 0 ]; then
    echo "PASS: netstack found the network scheme in every boot"
else
    echo "FAIL: netstack found the network scheme in every boot"
    exit 1
fi
