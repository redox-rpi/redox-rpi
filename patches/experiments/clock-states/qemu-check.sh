#!/usr/bin/env bash
# QEMU check for the clock-states experiment (#126): rpi-firmwared serves
# clock_states and keeps serving clocks. QEMU's firmware answers the clock
# tags in its own way, so the states themselves need the Pi.
#
# Usage: patches/experiments/clock-states/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
f=/scheme/rpi-firmware
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 600 \
    --run "ls $f; echo CS-BEGIN; cat $f/clock_states; echo CS-END; cat $f/clocks" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
if grep -q "clock_states" <<< "$log"; then echo "PASS: clock_states listed"; else echo "FAIL: clock_states listed"; failed=1; fi
states="$(sed -n '/^CS-BEGIN$/,/^CS-END$/p' <<< "$log" | grep -E '^[a-z0-9]+ (on|off)$')"
if [ -n "$states" ]; then echo "PASS: clock_states has lines"; else echo "INFO: clock_states is empty (QEMU's firmware has no clock states)"; fi
if grep -qE '^arm [0-9]+ [0-9]+$' <<< "$log"; then echo "PASS: clocks unchanged"; else echo "FAIL: clocks unchanged"; failed=1; fi
echo "--- clock_states:"
echo "$states"
exit $failed
