#!/usr/bin/env bash
# QEMU check for the eth-speed experiment (#126): dwc2d reads
# /tmp/dwc2d-link and keeps running. QEMU's USB network card is CDC
# Ethernet, not a LAN7800, so the request is refused there; switching the
# speed and the power it saves need the Pi.
#
# Usage: patches/experiments/eth-speed/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --usb-net --timeout 600 \
    --run "echo 100 > /tmp/dwc2d-link; sleep 3; echo fast > /tmp/dwc2d-link; sleep 3" \
    --run "echo ETH-AFTER; ls /tmp; ps" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
check() {
    if grep -qE "$2" <<< "$log"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
check "request read, no LAN7800 in QEMU" 'dwc2d: link: "100" needs 100 or 1000 and a LAN7800 in use'
check "a bad value is refused" 'dwc2d: link: "fast" needs 100 or 1000'
after="$(sed -n '/^ETH-AFTER$/,$p' <<< "$log")"
if grep -q "dwc2d-link" <<< "$after"; then echo "FAIL: request file removed"; failed=1; else echo "PASS: request file removed"; fi
if grep -q "dwc2d" <<< "$after"; then echo "PASS: dwc2d still running"; else echo "FAIL: dwc2d still running"; failed=1; fi
exit $failed
