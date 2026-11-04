#!/usr/bin/env bash
# QEMU check for the i2c experiment (#39): QEMU's raspi3b emulates the BSC
# controllers, and a DS1338 clock (the DS3231's time registers, plus 56
# bytes of RAM at 08 to 3f) sits on bus 1 at 0x68. Needs the gpio
# experiment in the image too.
#
# Usage: patches/experiments/i2c/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
year="$(date -u +%y)"
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 800 \
    --device ds1338,bus=i2c-bus.1,address=0x68 \
    --run "echo M2 \$(cat /scheme/gpio/2/mode) M3 \$(cat /scheme/gpio/3/mode)" \
    --run "cat /scheme/i2c/1/scan" \
    --run "i2c detect" \
    --run "echo Y \$(i2c get 68 6)" \
    --run "i2c set 68 8 12 34" --run "echo R \$(i2c get 68 8 2)" \
    --run "i2c get 50 0" \
    --run "cat /scheme/i2c/1/78" \
    --run "i2c dump 68" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
check() {
    if grep -qE "$2" <<< "$log"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
check "driver started" "bcm2835-i2cd: bus 1 at 0x3f804000, [0-9]+ kHz"
check "pins 2 and 3 alt0" "^M2 alt0 M3 alt0"
check "scan finds the clock" "^68$"
check "detect table" "^60: -- -- -- -- -- -- -- -- 68"
check "clock year (BCD)" "^Y $year$"
check "RAM written and read back" "^R 12 34$"
check "no device at 0x50" "i2c: read: No such device or address"
check "address 0x78 missing" "No such file or directory"
check "register dump" "^00: [0-9a-f]{2}( [0-9a-f]{2}){15}$"
exit $failed
