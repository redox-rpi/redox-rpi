#!/usr/bin/env bash
# QEMU check for the cpu-ondemand experiment (#23): with nothing running,
# the governor must see an idle system (target min); under a start loop it
# must go to max; cputop shows what keeps cores busy. QEMU's clock is fixed
# (min = max), so only the governor's decision can be checked, not the
# rate. Build with --experiment cputop too.
#
# Usage: patches/experiments/cpu-ondemand/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 900 \
    --run "sleep 20" \
    --run "echo IDLE; cat /scheme/rpi-firmware/governor" \
    --run "cputop 10" \
    --run "for i in 1..200; ls / > /dev/null; if test \$i -eq 100; echo LOAD; cat /scheme/rpi-firmware/governor; end; end" \
    --run "sleep 5; echo AFTER; cat /scheme/rpi-firmware/governor" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
after() { sed -n "/^$1\$/,/^min /p" <<< "$log"; }
check() {
    if grep -qE "$3" <<< "$2"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
check "governor running" "$log" "^mode ondemand$"
check "idle: target min" "$(after IDLE)" "^target min$"
check "load: target max" "$(after LOAD)" "^target max$"
check "after the load: target min again" "$(after AFTER)" "^target min$"
echo "--- governor and cputop:"
grep -E "^(IDLE|LOAD|AFTER)$|^target |^load " <<< "$log"
sed -n '/^cputop: /,/^$/p' <<< "$log" | head -16
exit $failed
