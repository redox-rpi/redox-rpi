#!/usr/bin/env bash
# QEMU check for the hdmi-audio experiment (#423). QEMU's raspi3b has no
# HDMI block, so bcm2835-hdmiaudiod plays its ring to the clock: this checks
# the start-up, audiod on top of it, the audiohw contract, the stats, tone
# and stop files, not the HDMI path.
#
# Usage: patches/experiments/hdmi-audio/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
H=/scheme/audiohw
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --data --timeout 800 \
    --run "ls /scheme | grep audio" \
    --run "cat $H/stats" \
    --run "tone -s 2 > /scheme/audio; echo TONE \$?" \
    --run "echo 1 440 > $H/tone; echo DTONE \$?" \
    --run "echo 99 > $H/tone; echo BADTONE \$?" \
    --run "cat $H/stats" \
    --run "echo 1 > $H/stop; echo STOP \$?" \
    --run "tone -s 1 > /scheme/audio; echo AFTER \$?" \
    --run "cat $H/stats" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
check() {
    if grep -qE "$2" <<< "$log"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
refute() {
    if grep -qE "$2" <<< "$log"; then echo "FAIL: $1"; failed=1; else echo "PASS: $1"; fi
}
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
check "driver started without HDMI" "bcm2835-hdmiaudiod: no HDMI output .*playing to the clock"
refute "PWM driver not started" "bcm2835-pwmaudiod:"
check "audiohw scheme" "^audiohw$"
check "audio scheme" "^audio$"
refute "audiod running (no error)" "audiod: .*(No such device|error)"
check "stats: clock mode" "^mode clock"
check "stats: sample rate" "^sample_rate 44100$"
check "tone through audiod" "^TONE 0$"
check "driver tone accepted" "^DTONE 0$"
refute "driver tone over 10 s accepted" "^BADTONE 0$"
check "driver tone logged" "bcm2835-hdmiaudiod: tone 440 Hz"
check "stats: loud chunks" "^loud_chunks [1-9]"
check "stats: tones 1" "^tones 1$"
check "stop" "^STOP 0$"
check "driver stopped" "bcm2835-hdmiaudiod: stopped"
check "audio after stop accepted" "^AFTER 0$"
check "stats: stopped" "^stopped$"
refute "panic" "panicked at"
exit $failed
