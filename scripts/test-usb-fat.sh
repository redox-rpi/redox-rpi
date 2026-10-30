#!/usr/bin/env bash
# Check USB sticks with a FAT filesystem in QEMU (experiment usb-fat): plug
# in a stick with an MBR and a FAT32 partition after boot; usbmountd mounts
# it as /scheme/usb0. Read a file from the host, write, rename and remove
# files, eject, unplug and check on the host that the files are there and the
# volume is marked clean. Plug it in again, read back, unplug without eject.
#
# Needs an image built with --experiment usb-hotplug --experiment
# usb-storage --experiment usb-fat, and mtools on the host. The host port
# for devshd is 12323, or FAT_PORT=<port>.
#
# Usage:
#   scripts/test-usb-fat.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
socket="$ROOT/images/qemu-monitor-fat.sock"
log="$ROOT/images/qemu-usb-fat.log"
stick="$ROOT/images/usb-fat-test.img"
work="$ROOT/images/usb-fat-test"
port=${FAT_PORT:-12323}
# The partition starts at 1 MiB.
part=1048576
failed=0

devsh() {
    python3 "$ROOT/scripts/devsh.py" 127.0.0.1 "$port" "$@" 2>/dev/null | tr -d '\r'
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

# Waits until the log has $1 at least $3 times (default once), $2 seconds.
wait_log() {
    local _
    for _ in $(seq "$2"); do
        [ "$(tr -d '\r' < "$log" | grep -acE "$1")" -ge "${3:-1}" ] && return 0
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

# Bit 0 of the FAT32 boot sector's byte 0x41 is the dirty flag.
clean() {
    [ $(( $(od -An -tu1 -j $((part + 0x41)) -N1 "$stick") & 1 )) -eq 0 ]
}

# A 64 MiB stick: MBR, one FAT32 partition from 1 MiB, with hello.txt.
rm -rf "$stick" "$work"
mkdir -p "$work"
truncate -s 64M "$stick"
echo 'start=2048, type=c' | sfdisk -q "$stick"
mformat -i "$stick@@$part" -F -v REDOXTEST ::
echo "hello from the host" > "$work/hello.txt"
mcopy -i "$stick@@$part" "$work/hello.txt" ::hello.txt
head -c 1048576 /dev/urandom > "$work/big.bin"
big_sha=$(sha256sum < "$work/big.bin" | cut -c1-64)

rm -f "$socket" "$log"
QEMU_DEVSH_PORT=$port timeout 900 "$ROOT/scripts/qemu-raspi3b.sh" --usb-net --monitor "$socket" \
    --image "$image" < /dev/null > "$log" 2>&1 &
qemu=$!
trap 'kill $qemu 2>/dev/null; sleep 1; pkill -P $qemu 2>/dev/null' EXIT

for _ in $(seq 60); do
    devsh "echo ready" | grep -q ready && break
    sleep 3
done

monitor "drive_add 0 if=none,id=stick,file=$stick,format=raw" \
    "device_add usb-storage,drive=stick,id=stick0" > /dev/null
check "stick mounted" wait_log "usbmountd: .*\(FAT\) on /scheme/usb0" 30
tr -d '\r' < "$log" | grep -aE "dwc2d: mass storage|usbmountd|fatfsd" | sed 's/^/      /'

check "read a file from the host" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'cat /scheme/usb0/hello.txt' 2>/dev/null | grep -aq 'hello from the host'"

out=$(devsh "cd /scheme/usb0
dd if=/scheme/rand of=rand.bin bs=4096 count=256
sha256sum rand.bin
mkdir dir
echo one > a.txt
echo two > b.txt
mv a.txt b.txt
mv /scheme/usb0/rand.bin /scheme/usb0/dir/moved.bin
echo gone > del.txt
rm del.txt
ls /scheme/usb0 /scheme/usb0/dir
cat b.txt
cd /
usbmountd eject usb0
ls /scheme/usb0")
echo "$out" > "$work/out.txt"
echo "$out" | grep -aE 'rand.bin|moved|b.txt|usbmountd|No such|^two' | sed 's/^/      /'
rand_sha=$(echo "$out" | grep -aoE '^[0-9a-f]{64}  rand.bin' | cut -c1-64)
check "rename replaces the target" grep -aqx 'two' "$work/out.txt"
check "eject" grep -aq 'usb0 unmounted' "$work/out.txt"
check "fatfsd unmounted" wait_log "fatfsd: unmounted" 5
check "scheme gone after eject" bash -c "! python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'ls /scheme' 2>/dev/null | grep -aqw usb0"

monitor "device_del stick0" > /dev/null
check "stick removed" wait_log "dwc2d: mass storage removed" 10

# On the host.
check "volume marked clean" clean
mdir -i "$stick@@$part" -b :: :: ::/dir 2>/dev/null | sed 's/^/      /'
check "moved file on the host, same SHA-256" bash -c "[ -n '$rand_sha' ] && [ \"\$(mtype -i '$stick@@$part' ::dir/moved.bin | sha256sum | cut -c1-64)\" = '$rand_sha' ]"
check "renamed file replaced the target" bash -c "[ \"\$(mtype -i '$stick@@$part' ::b.txt)\" = one ]"
check "a.txt and del.txt gone" bash -c "! mdir -i '$stick@@$part' -b :: 2>/dev/null | grep -aqiE 'a.txt|del.txt'"
check "host file untouched" bash -c "[ \"\$(mtype -i '$stick@@$part' ::hello.txt)\" = 'hello from the host' ]"
# A file the host puts on the stick, for the second round.
mcopy -i "$stick@@$part" "$work/big.bin" ::big.bin

# Plugged in again: mounted again.
monitor "drive_add 0 if=none,id=stick2,file=$stick,format=raw" \
    "device_add usb-storage,drive=stick2,id=stick1,port=1.3" > /dev/null
check "mounted again" wait_log "usbmountd: .*\(FAT\) on /scheme/usb0" 30 2
check "host's 1 MiB file reads back" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'sha256sum /scheme/usb0/big.bin' 2>/dev/null | grep -aq '$big_sha'"
devsh "echo three >> /scheme/usb0/b.txt" > /dev/null

# Unplugged without eject: fatfsd ends, the name is free again.
monitor "device_del stick1" > /dev/null
check "fatfsd ends when the stick goes away" wait_log "usbmountd: /scheme/usb0 ended" 15 2
check "scheme gone after unplugging" bash -c "! python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'ls /scheme' 2>/dev/null | grep -aqw usb0"
tr -d '\r' < "$log" | grep -aE "usbmountd|fatfsd" | tail -n 6 | sed 's/^/      /'
exit $failed
