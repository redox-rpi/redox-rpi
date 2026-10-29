#!/usr/bin/env bash
# Check USB serial adapters in dwc2d (experiment usb-serial) in QEMU,
# through QEMU's monitor: plug a usb-serial (an FTDI FT232) in after boot,
# with its other end on a Unix socket this script talks to; write to the
# port and read from it, set the speed, switch its port off and on, unplug
# it and plug it in again. dwc2d prints what goes through while
# /tmp/dwc2d-serial-log exists.
#
# Needs an image built with --experiment usb-hotplug --experiment
# usb-serial. The host port for devshd is 12323, or SERIAL_PORT=<port>.
#
# Usage:
#   scripts/test-usb-serial.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
socket="$ROOT/images/qemu-monitor-serial.sock"
line="$ROOT/images/qemu-serial-line.sock"
received="$ROOT/images/qemu-serial-received"
send="$ROOT/images/qemu-serial-send"
log="$ROOT/images/qemu-usb-serial.log"
port=${SERIAL_PORT:-12323}
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
    out = until_prompt().replace("\r", "")
    print("\n".join(l for l in out.split("\n")[1:] if not l.startswith("(qemu)")))
PYEOF
}

# The far end of the serial line: everything the adapter sends goes to
# $received; whatever is written to $send goes to the adapter.
far_end() {
    python3 - "$line" "$received" "$send" <<'PYEOF' &
import os, socket, sys, time
for _ in range(100):
    try:
        s = socket.socket(socket.AF_UNIX)
        s.connect(sys.argv[1])
        break
    except OSError:
        time.sleep(0.1)
s.settimeout(0.1)
out = open(sys.argv[2], "ab", buffering=0)
while True:
    try:
        data = s.recv(4096)
        if not data:
            break
        out.write(data)
    except socket.timeout:
        pass
    if os.path.exists(sys.argv[3]):
        data = open(sys.argv[3], "rb").read()
        os.remove(sys.argv[3])
        s.sendall(data)
PYEOF
    far=$!
}

plug() {
    monitor "chardev-add socket,id=line$1,path=$line,server=on,wait=off" > /dev/null
    far_end
    monitor "device_add usb-serial,chardev=line$1,id=serial0${2:+,port=$2}" > /dev/null
}

unplug() {
    monitor "device_del serial0" > /dev/null
    sleep 2
    kill "$far" 2>/dev/null
    monitor "chardev-remove line$1" > /dev/null
}

# The number of log lines matching $1.
count() {
    tr -d '\r' < "$log" | grep -acE "$1"
}

# Wait up to $2 seconds until more than $3 log lines match $1.
wait_more() {
    local _
    for _ in $(seq "$2"); do
        [ "$(count "$1")" -gt "$3" ] && return 0
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

# The port name dwc2d gave the last adapter it found.
port_name() {
    tr -d '\r' < "$log" | grep -aoE "dwc2d: serial /scheme/usb-serial/[0-9.]+" | tail -1 | sed 's|.*/||'
}

rm -f "$socket" "$line" "$received" "$send" "$log"
QEMU_DEVSH_PORT=$port timeout 600 "$ROOT/scripts/qemu-raspi3b.sh" --usb-net --monitor "$socket" \
    --image "$image" < /dev/null > "$log" 2>&1 &
qemu=$!
far=""
trap 'kill $far 2>/dev/null; kill $qemu 2>/dev/null; sleep 1; pkill -P $qemu 2>/dev/null' EXIT

for _ in $(seq 60); do
    devsh "echo ready" | grep -q ready && break
    sleep 3
done
devsh "touch /tmp/dwc2d-serial-log" > /dev/null
sleep 2

echo "== plug in"
n=$(count "dwc2d: serial /scheme/usb-serial/")
plug 0
check "adapter found" wait_more "dwc2d: serial /scheme/usb-serial/[0-9.]+: FTDI" 15 "$n"
name=$(port_name)
dev=/scheme/usb-serial/$name
tr -d '\r' < "$log" | grep -a "dwc2d: serial /scheme" | tail -1 | sed 's/^/      /'
check "port listed" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'ls /scheme/usb-serial' 2>/dev/null | grep -aq '$name'"

echo "== write and read"
devsh "echo hello-from-redox > $dev" > /dev/null
sleep 1
check "write reaches the far end" grep -aq "hello-from-redox" "$received"
# head blocks until the bytes come; they are sent while it waits.
(sleep 3; printf 'pong-from-far\n' > "$send") &
out=$(devsh "head -c 14 $dev")
check "read from the far end" grep -aq "pong-from-far" <<< "$out"

echo "== speed"
check "speed starts at 115200" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'cat $dev/baud' 2>/dev/null | grep -aq 115200"
n=$(count "dwc2d: serial $name: 9600 baud, 8N1")
devsh "echo 9600 > $dev/baud" > /dev/null
check "speed set to 9600" wait_more "dwc2d: serial $name: 9600 baud, 8N1" 5 "$n"
check "speed reads 9600" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'cat $dev/baud' 2>/dev/null | grep -aq 9600"
check "bad speed refused" bash -c "! python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'echo fast > $dev/baud && echo accepted' 2>/dev/null | grep -aq accepted"

echo "== port switched off and on"
# The test command for the real Pi; QEMU's hub powers its ports together,
# so dwc2d forgets and resets the device.
n=$(count "dwc2d: serial /scheme/usb-serial/[0-9.]+: FTDI")
devsh "echo serial > /tmp/dwc2d-cycle" > /dev/null
check "adapter found after the port cycle" wait_more "dwc2d: serial /scheme/usb-serial/[0-9.]+: FTDI" 15 "$n"
name=$(port_name)
dev=/scheme/usb-serial/$name
: > "$received"
devsh "echo after-cycle > $dev" > /dev/null
sleep 1
check "write after the port cycle" grep -aq "after-cycle" "$received"

echo "== unplug and plug in again"
n=$(count "dwc2d: serial [0-9.]+ removed")
unplug 0
check "adapter removed" wait_more "dwc2d: serial [0-9.]+ removed" 10 "$n"
check "port gone from the list" bash -c "! python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'ls /scheme/usb-serial' 2>/dev/null | grep -aq '$name'"
n=$(count "dwc2d: serial /scheme/usb-serial/[0-9.]+: FTDI")
: > "$received"
plug 1 1.3
check "adapter found again" wait_more "dwc2d: serial /scheme/usb-serial/[0-9.]+: FTDI" 15 "$n"
name=$(port_name)
dev=/scheme/usb-serial/$name
devsh "echo replugged > $dev" > /dev/null
sleep 1
check "write after plugging in again" grep -aq "replugged" "$received"

check "devshd still answers" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'echo ready' 2>/dev/null | grep -q ready"
echo
tr -d '\r' < "$log" | grep -aE "dwc2d: serial" | head -30 | sed 's/^/      /'
exit $failed
