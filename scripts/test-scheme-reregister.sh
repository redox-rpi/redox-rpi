#!/usr/bin/env bash
# #117: a scheme daemon that ends keeps its name, and registering the name
# again fails with "File exists". In QEMU: start fatfsd on the image's FAT
# partition (the first one fatfsd can mount) as scheme t117, kill it with
# SIGKILL (so it cannot remove its name), start it again under the same name
# and list its root through it.
#
# Needs an image with fatfsd (experiment usb-fat) and, for the fix,
# --experiment scheme-reregister. Without the fix the second fatfsd fails.
# The host port for devshd is 12324, or T117_PORT=<port>.
#
# Usage:
#   scripts/test-scheme-reregister.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
log="$ROOT/images/qemu-scheme-reregister.log"
port=${T117_PORT:-12324}
disk=
failed=0

# devsh output is a terminal's: drop escape sequences and the prompt lines.
devsh() {
    python3 "$ROOT/scripts/devsh.py" 127.0.0.1 "$port" "$@" 2>/dev/null | python3 -c '
import re, sys
s = sys.stdin.buffer.read().decode(errors="replace")
s = re.sub(r"\x1b\[[0-9;?]*[a-zA-Z]|\x1b\][^\x07]*\x07", "", s).replace("\x00", "")
for line in s.replace("\r", "\n").split("\n"):
    if line.strip() and "# " not in line and "login:" not in line and "password:" not in line:
        print(line)
'
}

check() {
    if "${@:2}"; then
        echo "PASS: $1"
    else
        echo "FAIL: $1"
        failed=1
    fi
}

listed() { devsh "ls -1 /scheme" | grep -aqx t117; }
# A listing of the FAT root without an error.
reads() { devsh "ls /scheme/t117 && echo LISTED" | grep -aq LISTED; }

rm -f "$log"
DEVSH_PORT=$port timeout 600 "$ROOT/scripts/qemu-raspi3b.sh" --usb-net --image "$image" \
    < /dev/null > "$log" 2>&1 &
qemu=$!
trap 'kill $qemu 2>/dev/null; sleep 1; pkill -P $qemu 2>/dev/null' EXIT

for _ in $(seq 60); do
    devsh "echo ready" | grep -q ready && break
    sleep 3
done

# Round 1: the FAT partition through fatfsd as /scheme/t117. fatfsd ends
# on a partition without FAT (BIOS boot, RedoxFS) and removes its name.
for part in 0p0 0p1 0p2; do
    devsh "fatfsd /scheme/disk.mmc/$part t117 &" "sleep 2" > /dev/null
    if reads; then
        disk=/scheme/disk.mmc/$part
        break
    fi
done
echo "      FAT partition: ${disk:-none}"
check "first fatfsd serves t117" test -n "$disk"

# SIGKILL: fatfsd cannot remove its name.
devsh "pkill -9 fatfsd" "sleep 1" > /dev/null
check "fatfsd gone" bash -c "! python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port ps 2>/dev/null | grep -aq fatfsd"
listed && echo "      t117 is still listed after the kill (the name stays until it is registered again)"

# Round 2: the same name again.
devsh "fatfsd $disk t117 &" "sleep 2" > /dev/null
check "second fatfsd serves t117 again" reads
check "no File exists" bash -c "! tr -d '\r' < '$log' | grep -aq 'File exists'"
tr -d '\r' < "$log" | grep -aE "fatfsd|nsmgr|File exists" | tail -n 6 | sed 's/^/      /'

devsh "pkill fatfsd" > /dev/null
exit $failed
