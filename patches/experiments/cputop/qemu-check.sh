#!/usr/bin/env bash
# QEMU check for the cputop experiment (#58): after boot, with nothing
# running, sample CPU time per core and per thread, and print who keeps a
# core busy. The busy threads are reported, not judged: in QEMU some
# drivers poll without pause.
#
# Usage: patches/experiments/cputop/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 800 \
    --run "sleep 20" \
    --run "cputop 10" \
    --run "cputop -n 2 -l 3 2" \
    --run "cputop -l 0 1" \
    --run "cputop x" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
check() {
    if grep -qE "$2" <<< "$log"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
count() {
    local n
    n="$(grep -cE "$2" <<< "$log")"
    if [ "$n" -eq "$3" ]; then echo "PASS: $1 ($n)"; else echo "FAIL: $1 ($n, not $3)"; failed=1; fi
}
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
check "header" "^cputop: 10\.[0-9] s, [0-9]+ threads$"
check "per core" "^cpu0 +busy +[0-9]+ %  \([0-9]+ ms busy, [0-9]+ ms idle\)$"
check "thread table" "^ +PID +CPU% +TOTAL STAT +CPU +NAME$"
check "thread rows" "^ +[0-9]+ +[0-9]+\.[0-9] +[0-9]+:[0-9]{2}:[0-9]{2}\.[0-9] "
count "rounds: 1 + 2 + 1" "^cputop: [0-9.]+ s, " 4
check "usage on a bad argument" "usage: cputop"
echo "--- idle after boot (cputop 10):"
sed -n '/^cputop: 10\./,/^$/p' <<< "$log" | head -18
exit $failed
