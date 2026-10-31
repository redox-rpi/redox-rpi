#!/usr/bin/env bash
# QEMU check for the boot-time CPU clock setting (#23): the image built with
# --experiment governor-test has /etc/rpi-firmware/governor = performance,
# so 15_rpi-governor.service sets that mode at boot. Then the same script by
# hand with a rate (goes to arm_clock), a bad word (refused by both) and
# ondemand.
#
# Usage: patches/experiments/governor-test/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
f=/scheme/rpi-firmware/governor
g=/etc/rpi-firmware/governor
s=/usr/lib/rpi-firmware/governor.ion
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 600 \
    --run "echo AT-BOOT; cat $f" \
    --run "echo 600000000 > $g; ion $s; echo AFTER-RATE; cat $f" \
    --run "echo bogus > $g; ion $s; echo AFTER-BOGUS; cat $f" \
    --run "echo ondemand > $g; ion $s; echo AFTER-ONDEMAND; cat $f" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
check() {
    if grep -qE "$2" <<< "$log"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
# The line after a marker, e.g. the governor file's first line.
after() { sed -n "/^$1\$/{n;p;q}" <<< "$log"; }
expect() {
    local got
    got="$(after "$2")"
    if grep -qE "$3" <<< "$got"; then echo "PASS: $1"; else echo "FAIL: $1 (got: $got)"; failed=1; fi
}
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
check "set at boot from /etc" "rpi-firmwared: governor performance"
expect "mode performance after boot" AT-BOOT "^mode performance"
check "a rate goes to arm_clock" "rpi-firmwared: ARM clock [0-9]+ MHz, governor off"
expect "mode fixed after a rate" AFTER-RATE "^mode fixed"
expect "a bad word changes nothing" AFTER-BOGUS "^mode fixed"
expect "back to ondemand" AFTER-ONDEMAND "^mode ondemand"
echo "--- governor lines:"
grep -E "rpi-firmwared: (governor|ARM clock)" <<< "$log" | head
exit $failed
