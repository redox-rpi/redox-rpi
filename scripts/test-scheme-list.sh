#!/usr/bin/env bash
# Check an image with the experiment ns-list-snapshot in QEMU: netstack
# found the network scheme at boot, /scheme lists every scheme once and in
# sorted order with ls -U (the listing now goes through a sorted snapshot;
# before, it came in hash map order), and the
# same listing twice in a row is equal.
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
DEVSH_PORT=$port timeout 600 "$ROOT/scripts/qemu-raspi3b.sh" --usb-net --image "$image" \
    < /dev/null > "$log" 2>&1 &
qemu=$!
# QEMU runs below the wrapper script: stop it by its devshd port.
trap 'pkill -f "[q]emu-system-aarch64.*hostfwd=tcp::$port-"; kill $qemu 2>/dev/null' EXIT

for _ in $(seq 80); do
    devsh "echo ready" | grep -q ready && break
    sleep 3
done
check "devshd answers" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'echo ready' 2>/dev/null | grep -q ready"
check "netstack found a network adapter" bash -c "! grep -aq 'no network adapter found' '$log'"

# One name per line, between two markers.
names() {
    devsh "echo BEGIN" "ls -U -1 /scheme" "echo END" | sed -n '/^BEGIN$/,/^END$/p' | grep -vxE 'BEGIN|END|.*ls -U -1 /scheme.*|echo END' | grep -a '^[a-z]'
}
first=$(names)
second=$(names)
echo "      $(grep -c . <<< "$first") schemes listed"
check "the network scheme is listed once" test "$(grep -c '^network\.' <<< "$first")" -eq 1
check "no scheme listed twice" test -z "$(sort <<< "$first" | uniq -d)"
check "the listing is sorted" test "$first" = "$(LC_ALL=C sort <<< "$first")"
check "two listings are equal" test "$first" = "$second"
exit $failed
