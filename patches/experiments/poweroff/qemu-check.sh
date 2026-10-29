#!/usr/bin/env bash
# QEMU check for the poweroff experiment (#7): `poweroff` stops the programs
# started after boot, closes the file systems and halts through the
# watchdog; QEMU models the halt (partition 63) and exits instead of
# restarting. The run script then reports "QEMU exited", which is the
# expected end here.
#
# Usage: patches/experiments/poweroff/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 800 \
    --run "poweroff" > "$ROOT/images/qemu-poweroff.out" 2>&1
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
check() {
    if grep -qE "$2" <<< "$log"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
check "programs stopped" "^poweroff: [1-9][0-9]* programs stopped, [0-9]+ killed"
check "file systems closed" "^poweroff: [1-9][0-9]* file systems closed"
check "kernel stop" "kstop"
if grep -q "QEMU exited" "$ROOT/images/qemu-poweroff.out"; then echo "PASS: QEMU halted"; else echo "FAIL: QEMU halted"; failed=1; fi
if grep -qE "did not shutdown|poweroff: .*(error|denied)" <<< "$log"; then echo "FAIL: poweroff error"; failed=1; fi
# After kstop nothing may start again (a reset would show U-Boot twice).
if [ "$(grep -c "U-Boot 20" <<< "$log")" -le 1 ]; then echo "PASS: no restart"; else echo "FAIL: no restart"; failed=1; fi
exit $failed
