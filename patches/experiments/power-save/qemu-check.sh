#!/usr/bin/env bash
# QEMU check for the power-save experiment (#126): rpi-firmwared's display
# and power_domains files read, take writes, and refuse to switch off the
# domains in use (USB, PL011 console). QEMU's firmware has no real power
# domains or HDMI, so this checks the driver, not the saving.
#
# Usage: patches/experiments/power-save/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
f=/scheme/rpi-firmware
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 600 \
    --run "cat $f/display; cat $f/power_domains" \
    --run "echo usb off > $f/power_domains; echo uart0 off > $f/power_domains" \
    --run "echo i2c2 off > $f/power_domains; echo i2c2 on > $f/power_domains" \
    --run "echo off > $f/display; echo on > $f/display; echo blank > $f/display; echo unblank > $f/display" \
    --run "echo PD-AFTER; cat $f/display; cat $f/power_domains" \
    --run "echo bogus > $f/display" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
check() {
    if grep -qE "$2" <<< "$log"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
check "display reads on at start" "^on$"
# QEMU's firmware marks every domain as missing, so the list is empty
# there; on the Pi it is checked by the Pi test.
if grep -qE "^(sd|uart0|uart1|usb|i2c0|i2c1|i2c2|spi|ccp2tx) (on|off)$" <<< "$log"; then
    echo "PASS: power domains listed"
else
    echo "INFO: no power domains listed (none in QEMU's firmware)"
fi
check "usb refused" "power domain usb is in use, not switched off"
check "uart0 refused" "power domain uart0 is in use, not switched off"
check "i2c2 switched off" "rpi-firmwared: power domain i2c2 off \(firmware answered"
check "i2c2 switched on" "rpi-firmwared: power domain i2c2 on \(firmware answered"
check "display off" "rpi-firmwared: display off \(firmware answered"
check "display on" "rpi-firmwared: display on \(firmware answered"
check "display blank" "rpi-firmwared: display blank \(firmware answered"
after="$(sed -n '/^PD-AFTER$/,$p' <<< "$log")"
if grep -qx unblank <<< "$after"; then echo "PASS: display reads the last setting"; else echo "FAIL: display reads the last setting"; failed=1; fi
check "a bad value is refused" "Invalid argument"
echo "--- power_domains:"
sed -n '/^PD-AFTER$/,/^$/p' <<< "$log" | head -12
exit $failed
