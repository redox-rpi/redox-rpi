#!/usr/bin/env bash
# QEMU check for sound in SDL games (#335): Sopwith on the desktop plays its
# title tune through SDL2's Redox audio driver, audiod and
# bcm2835-pwmaudiod (which plays to the clock in QEMU). Needs an image with
# the desktop experiments:
#   scripts/build-image.sh --experiment timesync --experiment rpi-fb \
#     --experiment orbital --experiment orbital-apps \
#     --experiment cosmic-apps-source --experiment desktop
#
# Usage: patches/experiments/pwm-audio/qemu-sdl-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
H=/scheme/audiohw
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 900 \
    --run "desktop --yes" \
    --run "grep loud_chunks $H/stats" \
    --run "sopwith &> /tmp/sop.log &" \
    --run "sleep 15" \
    --run "cat /tmp/sop.log" \
    --run "ps | grep sopwith" \
    --run "grep loud_chunks $H/stats; grep peak $H/stats" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
check() {
    if grep -qE "$2" <<< "$log"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
check "Sopwith opened a window" "Initializing orbital window"
check "SDL chose the Redox audio driver" "You are using the SDL redox audio driver"
check "Sopwith still runs" "/usr/bin/sopwith"
grep -E "^loud_chunks [0-9]+$" <<< "$log" | awk '{print $2}' > /tmp/sdl-loud.$$
before=$(head -1 /tmp/sdl-loud.$$); after=$(tail -1 /tmp/sdl-loud.$$); rm -f /tmp/sdl-loud.$$
echo "loud_chunks before Sopwith: ${before:-?}, after: ${after:-?}"
if [ -n "$before" ] && [ -n "$after" ] && [ "$after" -gt "$before" ]; then
    echo "PASS: the title tune reached the driver"
else
    echo "FAIL: the title tune reached the driver"; failed=1
fi
exit $failed
