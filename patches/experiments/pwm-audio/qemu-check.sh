#!/usr/bin/env bash
# QEMU check for the pwm-audio experiment (#250). QEMU's raspi3b does not
# emulate the PWM block, so bcm2835-pwmaudiod plays its ring to the clock:
# this checks the start-up, audiod on top of it, the audiohw contract and the
# stats file, not the sound.
#
# Usage: patches/experiments/pwm-audio/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
H=/scheme/audiohw
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 800 \
    --run "ls /scheme | grep audio" \
    --run "cat $H/stats" \
    --run "tone check" \
    --run "tone -s 2 > /scheme/audio; echo TONE \$?" \
    --run "tone -c left -f 440 -s 1 -o /tmp/l.raw; ls -l /tmp/l.raw" \
    --run "cat /tmp/l.raw > /scheme/audio; echo CAT \$?" \
    --run "echo 1 > $H/tone" \
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
check "driver started without PWM clock" "bcm2835-pwmaudiod: .*\(QEMU\?\), playing to the clock"
check "audiohw and audio schemes" "^audiohw$"
check "audio scheme" "^audio$"
refute "audiod running (no error)" "audiod: .*(No such device|error)"
check "stats: clock mode" "^mode clock"
check "stats: range 2834" "^range 2834$"
check "short write EINVAL" "short write: EINVAL"
check "full ring EWOULDBLOCK" "accepted before EWOULDBLOCK"
check "tone through audiod" "^TONE 0$"
check "raw file 176400 bytes" "176400 .*/tmp/l.raw"
check "cat through audiod" "^CAT 0$"
check "CPU tone refused without PWM" "No such device"
check "chunks arrive" "^chunks [1-9][0-9]+"
check "rate about 44.1 kHz" "^measured_rate_hz 4[34][0-9]{3}$"
check "no DMA faults" "^dma_faults 0$"
exit $failed
