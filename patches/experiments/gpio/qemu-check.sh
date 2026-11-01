#!/usr/bin/env bash
# QEMU check for the gpio experiment (#38): QEMU's raspi3b emulates the GPIO
# block (function select, set, clear, level), so modes and output levels can
# be checked; pulls have no effect there, and QEMU is a Pi 3B without the
# 3B+ power LED.
#
# Usage: patches/experiments/gpio/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
G=/scheme/gpio
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 800 \
    --run "ls $G/17" \
    --run "echo out > $G/17/mode" --run "echo 1 > $G/17/value" --run "echo M17 \$(cat $G/17/mode) V17 \$(cat $G/17/value)" \
    --run "echo 0 > $G/17/value" --run "echo V17b \$(cat $G/17/value)" \
    --run "echo alt0 > $G/4/mode" --run "echo M4 \$(cat $G/4/mode)" \
    --run "echo up > $G/17/pull" --run "echo P17 \$(cat $G/17/pull) P18 \$(cat $G/18/pull)" \
    --run "echo out > $G/14/mode" --run "echo bogus > $G/17/mode" --run "cat $G/60/mode" \
    --run "cat $G/pins" \
    --run "cat /scheme/rpi-firmware/power_led" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
check() {
    if grep -qE "$2" <<< "$log"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
check "driver started" "bcm2835-gpiod: 54 pins at 0x3f200000"
check "pin directory" "^mode +pull +value"
check "pin 17 output high" "^M17 out V17 1"
check "pin 17 output low" "^V17b 0"
check "pin 4 alt0" "^M4 alt0"
check "pull remembered" "^P17 up P18 unknown"
check "console pin refused" "Operation not permitted"
check "bad mode refused" "Invalid argument"
check "pin 60 missing" "No such file or directory"
check "pins table" "^17 out 0$"
exit $failed
