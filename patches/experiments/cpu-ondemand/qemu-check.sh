#!/usr/bin/env bash
# QEMU check for the cpu-ondemand experiment (#23). Reading the governor
# file starts a program, which is load itself, so the check does not look
# at `target` but at the counters: over 20 quiet seconds the clock must be
# at the minimum most of the time, and during a start loop at the maximum.
# QEMU's clock is fixed (min = max), so only the governor's decisions are
# checked, not the rate. Build with --experiment cputop too.
#
# Usage: patches/experiments/cpu-ondemand/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 900 \
    --run "sleep 20" \
    --run "echo G1; cat /scheme/rpi-firmware/governor" \
    --run "sleep 20" \
    --run "echo G2; cat /scheme/rpi-firmware/governor" \
    --run "for i in 1..100; ls / > /dev/null; end" \
    --run "echo G3; cat /scheme/rpi-firmware/governor" \
    --run "cputop 10" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
field() { sed -n "/^$1\$/,/^time_max_ms /p" <<< "$log" | awk -v k="$2" '$1 == k { print $2 }' | tail -1; }
pass() { echo "PASS: $1"; }
fail() { echo "FAIL: $1"; failed=1; }
[ $status -eq 0 ] || fail "QEMU run ($status)"
grep -q "^mode ondemand$" <<< "$log" && pass "governor running" || fail "governor running"
for g in G1 G2 G3; do
    for k in time_min_ms time_max_ms; do
        printf -v "${g}_$k" '%s' "$(field "$g" "$k")"
    done
done
if [ -n "$G1_time_min_ms" ] && [ -n "$G2_time_min_ms" ] && [ -n "$G3_time_max_ms" ]; then
    quiet_min=$((G2_time_min_ms - G1_time_min_ms))
    quiet_max=$((G2_time_max_ms - G1_time_max_ms))
    busy_min=$((G3_time_min_ms - G2_time_min_ms))
    busy_max=$((G3_time_max_ms - G2_time_max_ms))
    echo "quiet 20 s: ${quiet_min} ms at min, ${quiet_max} ms at max"
    echo "start loop: ${busy_min} ms at min, ${busy_max} ms at max"
    [ $((quiet_min * 100)) -ge $(((quiet_min + quiet_max) * 70)) ] \
        && pass "quiet: at least 70 % of the time at min" || fail "quiet: at least 70 % of the time at min"
    [ $((busy_max * 100)) -ge $(((busy_min + busy_max) * 70)) ] \
        && pass "start loop: at least 70 % of the time at max" || fail "start loop: at least 70 % of the time at max"
else
    fail "time_min_ms/time_max_ms in the governor file"
fi
echo "--- cputop:"
sed -n '/^cputop: /,/^$/p' <<< "$log" | head -16
exit $failed
