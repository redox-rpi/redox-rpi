#!/usr/bin/env bash
# QEMU check for lsusb and hwinfo (#35): both over devsh,
# then a second keyboard plugged in and out through QEMU's monitor; lsusb
# must follow. Needs an image with base 0053 and the hwinfo package.
#
# Usage: patches/experiments/hwinfo/qemu-check.sh [image] [devsh port]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
port="${2:-12377}"
socket="$ROOT/images/qemu-hwinfo-monitor.sock"
log="$ROOT/images/qemu-hwinfo.log"
failed=0

devsh() {
    python3 "$ROOT/scripts/devsh.py" 127.0.0.1 "$port" "$@" 2>/dev/null | tr -d '\r'
}

monitor() {
    python3 - "$socket" "$1" <<'PYEOF'
import socket, sys, time
mon = socket.socket(socket.AF_UNIX)
mon.connect(sys.argv[1])
mon.settimeout(5)
time.sleep(0.5)
mon.sendall((sys.argv[2] + "\n").encode())
time.sleep(1)
PYEOF
}

check() {
    if "${@:2}"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}

keyboards() {
    devsh "lsusb" | grep -c "keyboard"
}

rm -f "$socket" "$log"
DEVSH_PORT=$port timeout 600 "$ROOT/scripts/qemu-raspi3b.sh" --usb-net --monitor "$socket" --image "$image" \
    < /dev/null > "$log" 2>&1 &
qemu=$!
trap 'kill $qemu 2>/dev/null; sleep 2; pkill -P $qemu 2>/dev/null' EXIT

for _ in $(seq 60); do
    devsh "echo ready" | grep -q ready && break
    sleep 3
done

echo "== lsusb"; devsh "lsusb" | tee "$ROOT/images/hwinfo-lsusb.txt"
echo "== lsusb -t"; devsh "lsusb -t"
echo "== hwinfo"; devsh "hwinfo" | tee "$ROOT/images/hwinfo-hwinfo.txt"
check "lsusb lists the hub" grep -q "0409:55aa" "$ROOT/images/hwinfo-lsusb.txt"
check "hwinfo shows 4 CPUs" grep -q "^CPUs *4" "$ROOT/images/hwinfo-hwinfo.txt"

before=$(keyboards)
monitor "device_add usb-kbd,id=hwinfokbd"
sleep 6
plugged=$(keyboards)
echo "== lsusb after plugging a keyboard in"; devsh "lsusb"
check "lsusb shows the new keyboard ($before -> $plugged)" [ "$plugged" -eq $((before + 1)) ]
monitor "device_del hwinfokbd"
sleep 6
unplugged=$(keyboards)
check "lsusb drops it again ($plugged -> $unplugged)" [ "$unplugged" -eq "$before" ]

exit $failed
