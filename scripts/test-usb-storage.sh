#!/usr/bin/env bash
# Check USB sticks in dwc2d (experiment usb-storage) in QEMU: plug a stick
# in through QEMU's monitor after boot, read its first block through the
# disk scheme, copy it to block 2048, check the copy in the image on the
# host, unplug the stick, and plug it in again. Then a card reader (QEMU's
# usb-bot) with cards in slots 0 and 2 and an empty slot 1, next to the
# stick: both are read, the reader is unplugged, the stick still works.
#
# Needs an image built with --experiment usb-storage --experiment
# usb-storage-more (hotplug is base 0031-0033 on usb-hotplug). devshd port
# 12323, or STORAGE_PORT=<port>.
#
# Usage:
#   scripts/test-usb-storage.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
socket="$ROOT/images/qemu-monitor-storage.sock"
log="$ROOT/images/qemu-usb-storage.log"
stick="$ROOT/images/usb-stick-test.img"
card0="$ROOT/images/usb-card0-test.img"
card2="$ROOT/images/usb-card2-test.img"
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

# devsh output is a terminal's: escape sequences around the lines, and a
# raw block's NUL bytes would go through it too, so only look at the start.
check "scheme lists the whole disk" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'ls -l $scheme/' 2>/dev/null | grep -aqE ' 16777216 .* 0\b'"
check "first block read" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'dd if=$scheme/0 bs=512 count=1 | head -c ${#marker}' 2>/dev/null | grep -aq '$marker'"
# The disk scheme opens a disk once for writing at a time, so the copy goes
# through a file.
devsh "dd if=$scheme/0 of=/tmp/block0 bs=512 count=1; dd if=/tmp/block0 of=$scheme/0 bs=512 seek=2048 conv=notrunc" > /dev/null
sleep 1

monitor "device_del stick0" > /dev/null
check "stick removed" wait_log "dwc2d: mass storage /scheme/disk.* removed" 10
check "disk gone" bash -c "! python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'ls -l $scheme/' 2>/dev/null | grep -aqE ' 16777216 .* 0\b'"

# QEMU writes through to the image file.
check "block 2048 written" bash -c "dd if='$stick' bs=512 skip=2048 count=1 status=none | grep -aq '$marker'"

# The same port (QEMU's hub is on root port 1) again: the scheme gets the
# same name, which needs the old one gone from the namespace.
monitor "drive_add 0 if=none,id=stick2,file=$stick,format=raw" \
    "device_add usb-storage,drive=stick2,id=stick1,port=1.3" > /dev/null
check "stick found again" bash -c "for _ in \$(seq 30); do [ \$(tr -d '\r' < '$log' | grep -ac 'dwc2d: mass storage /scheme/.*/0:') -ge 2 ] && exit 0; sleep 1; done; exit 1"
check "same scheme name" bash -c "[ \$(tr -d '\r' < '$log' | grep -ac 'dwc2d: mass storage $scheme/0:') -ge 2 ]"
check "block 2048 read after plugging in again" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'dd if=$scheme/0 bs=512 skip=2048 count=1 | head -c ${#marker}' 2>/dev/null | grep -aq '$marker'"

# A card reader next to the stick: LUN 0 and 2 have a card (8 and 4 MiB),
# LUN 1 is a CD drive without a disc (an empty slot). QEMU attaches the
# reader only once all its LUNs are there.
echo "== card reader"
truncate -s 0 "$card0" "$card2"
truncate -s 8M "$card0"
truncate -s 4M "$card2"
printf '%s' "CARD0-$marker" | dd of="$card0" conv=notrunc status=none
printf '%s' "CARD2-$marker" | dd of="$card2" conv=notrunc status=none
monitor "device_add usb-bot,id=reader,attached=false" \
    "drive_add 0 if=none,id=card0,file=$card0,format=raw" \
    "device_add scsi-hd,bus=reader.0,lun=0,drive=card0" \
    "drive_add 0 if=none,id=cd1" \
    "device_add scsi-cd,bus=reader.0,lun=1,drive=cd1" \
    "drive_add 0 if=none,id=card2,file=$card2,format=raw" \
    "device_add scsi-hd,bus=reader.0,lun=2,drive=card2" \
    "qom-set reader attached true" > /dev/null
check "reader slot 2 found" wait_log "dwc2d: mass storage /scheme/disk\.usb-dwc2\+[0-9.]+-scsi/2: .*, 4 MiB" 30
reader=$(tr -d '\r' < "$log" | grep -aoE "/scheme/disk\.usb-dwc2\+[0-9.]+-scsi/2:" | tail -1 | sed 's|/2:$||')
tr -d '\r' < "$log" | grep -aE "dwc2d: mass storage" | tail -4 | sed 's/^/      /'
# The scheme name as a pattern: "+" and "." are taken literally.
reader_re=$(printf '%s' "$reader" | sed 's/[.+]/\\&/g')
check "reader slot 0 found" bash -c "tr -d '\r' < '$log' | grep -aqE 'dwc2d: mass storage $reader_re/0: .*, 8 MiB'"
# QEMU answers REQUEST SENSE for a LUN other than 0 with "LUN not
# supported" instead of "medium not present", so the slot is left out only
# after the 5 s a stick may take to get ready; a real reader says "no
# medium" at once.
check "empty slot 1 left out" bash -c "tr -d '\r' < '$log' | grep -aqE 'LUN 1: Timeout' && ! tr -d '\r' < '$log' | grep -aqE 'mass storage $reader_re/1:'"
check "reader slot 0 read" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'dd if=$reader/0 bs=512 count=1 | head -c 64' 2>/dev/null | grep -aq 'CARD0-$marker'"
check "reader slot 2 read" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'dd if=$reader/2 bs=512 count=1 | head -c 64' 2>/dev/null | grep -aq 'CARD2-$marker'"
check "stick read next to the reader" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'dd if=$scheme/0 bs=512 skip=2048 count=1 | head -c ${#marker}' 2>/dev/null | grep -aq '$marker'"
monitor "device_del reader" > /dev/null
check "reader removed" wait_log "dwc2d: mass storage $reader_re removed" 10
check "stick still read after the reader is gone" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'dd if=$scheme/0 bs=512 skip=2048 count=1 | head -c ${#marker}' 2>/dev/null | grep -aq '$marker'"
check "devshd still answers" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'echo ready' 2>/dev/null | grep -q ready"
exit $failed
