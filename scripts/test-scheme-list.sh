#!/usr/bin/env bash
# Check in QEMU that every listing of /scheme is complete while schemes are
# registered meanwhile. Needs an image built with --experiment
# scheme-list-stress: a test unit in init's namespace starts 40 ramfs
# instances and lists /scheme 200 times in between. Without the experiment
# ns-list-snapshot some listings miss the network scheme (netstack at boot
# then sometimes finds no network adapter); with it none should.
#
# The host port for devshd is 12323, or DEVSH_PORT=<port>.
#
# Usage:
#   scripts/test-scheme-list.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
log="$ROOT/images/qemu-scheme-list.log"
port=${DEVSH_PORT:-12323}
failed=0

devsh() {
    python3 "$ROOT/scripts/devsh.py" 127.0.0.1 "$port" "$@" 2>/dev/null | tr -d '\r\000'
}

check() {
    if "${@:2}"; then
        echo "PASS: $1"
    else
        echo "FAIL: $1"
        failed=1
    fi
}

rm -f "$log"
DEVSH_PORT=$port timeout 900 "$ROOT/scripts/qemu-raspi3b.sh" --usb-net --image "$image" \
    < /dev/null > "$log" 2>&1 &
qemu=$!
# QEMU runs below the wrapper script: stop it by its devshd port.
trap 'pkill -f "[q]emu-system-aarch64.*hostfwd=tcp::$port-"; kill $qemu 2>/dev/null' EXIT

for _ in $(seq 80); do
    devsh "echo ready" | grep -q ready && break
    sleep 3
done
check "devshd answers" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'echo ready' 2>/dev/null | grep -q ready"

for _ in $(seq 60); do
    devsh "cat /tmp/list-test.done" | grep -qE '^[0-9]+$' && break
    sleep 5
done
registered=$(devsh "cat /tmp/list-test.done" | grep -E '^[0-9]+$' | tail -1)
counts=$(devsh "cat /tmp/list-test.out" | grep -E '^[0-9]+$')
listings=$(grep -c . <<< "$counts")
missed=$(grep -cvx 1 <<< "$counts")
echo "      ${registered:-0} schemes registered during $listings listings, $missed listings without exactly one network scheme"
check "the test unit ran (40 schemes registered)" test "${registered:-0}" -eq 40
check "200 listings" test "$listings" -eq 200
check "every listing has the network scheme once" test "$missed" -eq 0
exit $failed
