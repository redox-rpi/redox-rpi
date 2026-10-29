#!/usr/bin/env bash
# Check USB sticks in dwc2d (experiment usb-storage) in QEMU: plug a stick
# in through QEMU's monitor after boot, read its first block through the
# disk scheme, copy it to block 2048, check the copy in the image on the
# host, and unplug the stick again.
#
# Needs an image built with --experiment usb-hotplug --experiment
# usb-storage. The host port for devshd is 12323, or STORAGE_PORT=<port>.
#
# Usage:
#   scripts/test-usb-storage.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
socket="$ROOT/images/qemu-monitor-storage.sock"
log="$ROOT/images/qemu-usb-storage.log"
stick="$ROOT/images/usb-stick-test.img"
port=${STORAGE_PORT:-12323}
marker="REDOX-RPI-USB-STICK-TEST-$$"
failed=0

devsh() {
    python3 "$ROOT/scripts/devsh.py" 127.0.0.1 "$port" "$@" 2>/dev/null
}

monitor() {
    python3 - "$socket" "$@" <<'PYEOF'
import socket, sys
mon = socket.socket(socket.AF_UNIX)
mon.connect(sys.argv[1])
mon.settimeout(5)
def until_prompt():
    data = b""
    while not data.endswith(b"(qemu) "):
        chunk = mon.recv(4096)
        if not chunk:
            break
        data += chunk
    return data.decode(errors="replace")
until_prompt()
for cmd in sys.argv[2:]:
    mon.sendall(cmd.encode() + b"\n")
    print(until_prompt().replace("\r", "").split("\n", 1)[-1])
PYEOF
}

wait_log() {
    local _
    for _ in $(seq "$2"); do
        tr -d '\r' < "$log" | grep -aqE "$1" && return 0
        sleep 1
    done
    return 1
}

check() {
    if "${@:2}"; then
        echo "PASS: $1"
    else
        echo "FAIL: $1"
        failed=1
    fi
}

# A 16 MiB stick with the marker at the start of block 0.
rm -f "$stick"
truncate -s 16M "$stick"
printf '%s' "$marker" | dd of="$stick" conv=notrunc status=none

rm -f "$socket" "$log"
QEMU_DEVSH_PORT=$port timeout 600 "$ROOT/scripts/qemu-raspi3b.sh" --usb-net --monitor "$socket" \
    --image "$image" < /dev/null > "$log" 2>&1 &
qemu=$!
trap 'kill $qemu 2>/dev/null; sleep 1; pkill -P $qemu 2>/dev/null' EXIT

for _ in $(seq 60); do
    devsh "echo ready" | grep -q ready && break
    sleep 3
done

monitor "drive_add 0 if=none,id=stick,file=$stick,format=raw" \
    "device_add usb-storage,drive=stick,id=stick0" > /dev/null
check "stick found" wait_log "dwc2d: mass storage /scheme/disk\.usb-dwc2" 30
scheme=$(tr -d '\r' < "$log" | grep -aoE "/scheme/disk\.usb-dwc2\+[0-9.]+-scsi" | head -1)
echo "      $scheme"
tr -d '\r' < "$log" | grep -aE "dwc2d: mass storage" | sed 's/^/      /'

check "scheme lists the whole disk" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'ls $scheme' 2>/dev/null | grep -qx '0'"
check "first block read" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'dd if=$scheme/0 bs=512 count=1' 2>/dev/null | grep -aq '$marker'"
devsh "dd if=$scheme/0 of=$scheme/0 bs=512 count=1 seek=2048" > /dev/null
sleep 1

monitor "device_del stick0" > /dev/null
check "stick removed" wait_log "dwc2d: mass storage removed" 10
check "scheme gone" bash -c "! python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'ls /scheme' 2>/dev/null | grep -q 'disk.usb-dwc2'"

# QEMU writes through to the image file.
check "block 2048 written" bash -c "dd if='$stick' bs=512 skip=2048 count=1 status=none | grep -aq '$marker'"
exit $failed
