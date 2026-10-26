#!/usr/bin/env bash
# Check the USB keyboard driver in QEMU: type on QEMU's USB keyboard at the
# login prompt, then look for the file the typed commands created, through
# devshd. Needs an image built with scripts/build-image.sh (network boot).
#
# Usage:
#   scripts/test-keyboard.sh [image]

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
socket="$ROOT/images/qemu-monitor.sock"
log="$ROOT/images/qemu-keyboard.log"
rm -f "$socket" "$log"

timeout 300 "$ROOT/scripts/qemu-raspi3b.sh" --usb-net --monitor "$socket" --image "$image" \
    < /dev/null > "$log" 2>&1 &
qemu=$!
trap 'kill $qemu 2>/dev/null || true' EXIT

devsh() {
    python3 "$ROOT/scripts/devsh.py" 127.0.0.1 12323 "$@" 2>/dev/null
}

# devshd starts late, after the console's login prompt.
for _ in $(seq 60); do
    devsh "echo ready" | grep -q ready && break
    sleep 3
done

type_text() {
    python3 - "$socket" "$1" <<'PYEOF'
import socket, sys, time
names = {" ": "spc", "\n": "ret", "/": "slash", ".": "dot", "-": "minus"}
mon = socket.socket(socket.AF_UNIX)
mon.connect(sys.argv[1])
for ch in sys.argv[2]:
    mon.sendall(f"sendkey {names.get(ch, ch)}\n".encode())
    time.sleep(0.15)
time.sleep(1)
PYEOF
}

type_text $'root\n'
sleep 2
type_text $'password\n'
sleep 4
type_text $'touch /home/root/typed-on-usb-keyboard\n'
sleep 3

if devsh "ls /home/root" | grep -q typed-on-usb-keyboard; then
    echo "PASS: typed on the USB keyboard"
else
    # What the screen shows.
    python3 - "$socket" "$ROOT/images/qemu-keyboard.ppm" <<'PYEOF'
import socket, sys, time
mon = socket.socket(socket.AF_UNIX)
mon.connect(sys.argv[1])
mon.sendall(f"screendump {sys.argv[2]}\n".encode())
time.sleep(2)
PYEOF
    echo "FAIL: no file from the USB keyboard, see $log and $ROOT/images/qemu-keyboard.ppm"
    exit 1
fi
