#!/usr/bin/env bash
# Check USB mice and tablets in dwc2d (experiment usb-mouse) in QEMU,
# through QEMU's monitor: plug a usb-mouse in after boot, move it, press a
# button, turn the wheel, switch its port off and on, unplug it; then the
# same for a usb-tablet (absolute positions). dwc2d prints the events it
# sends to inputd while /tmp/dwc2d-input-log exists.
#
# Needs an image built with --experiment usb-mouse. The host port for
# devshd is 12323, or MOUSE_PORT=<port>.
#
# Usage:
#   scripts/test-usb-mouse.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
socket="$ROOT/images/qemu-monitor-mouse.sock"
qmp="$ROOT/images/qemu-qmp-mouse.sock"
log="$ROOT/images/qemu-usb-mouse.log"
port=${MOUSE_PORT:-12323}
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

# Absolute pointer position, 0..=32767 on both axes (QMP; the monitor's
# mouse_move only reaches relative devices).
abs_move() {
    python3 - "$qmp" "$1" "$2" <<'PYEOF'
import json, socket, sys
qmp = socket.socket(socket.AF_UNIX)
qmp.connect(sys.argv[1])
f = qmp.makefile("rw")
f.readline()
def call(cmd, args=None):
    f.write(json.dumps({"execute": cmd, **({"arguments": args} if args else {})}) + "\n")
    f.flush()
    while True:
        reply = json.loads(f.readline())
        if "return" in reply or "error" in reply:
            return reply
call("qmp_capabilities")
x, y = int(sys.argv[2]), int(sys.argv[3])
call("input-send-event", {"events": [
    {"type": "abs", "data": {"axis": "x", "value": x}},
    {"type": "abs", "data": {"axis": "y", "value": y}}]})
PYEOF
}

rel_move() {
    monitor "mouse_move $1 $2"
}

# Make the named pointer the one QEMU sends movements to.
mouse_set() {
    local index
    index=$(monitor "info mice" | sed 's/\x1b\[[0-9;]*[a-zA-Z]//g' | grep -a "$1" | grep -oE "Mouse #[0-9]+" | grep -oE "[0-9]+" | head -1)
    [ -n "$index" ] && monitor "mouse_set $index" > /dev/null
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

rm -f "$socket" "$qmp" "$log"
DEVSH_PORT=$port timeout 600 "$ROOT/scripts/qemu-raspi3b.sh" --usb-net --monitor "$socket" --qmp "$qmp" \
    --image "$image" < /dev/null > "$log" 2>&1 &
qemu=$!
trap 'kill $qemu 2>/dev/null; sleep 1; pkill -P $qemu 2>/dev/null' EXIT

for _ in $(seq 60); do
    devsh "echo ready" | grep -q ready && break
    sleep 3
done
devsh "touch /tmp/dwc2d-input-log" > /dev/null

# A pointer: $1 the QEMU device, $2 its name in "info mice", $3 relative or
# absolute, $4 the event a movement gives, $5 and $6 two movements: a
# command and its arguments.
pointer() {
    local device=$1 name=$2 kind=$3 moved=$4 move1=$5 move2=$6 n
    echo "== $device"
    n=$(count "dwc2d: mouse at address [0-9]+: $kind")
    monitor "device_add $device,id=pointer0" > /dev/null
    check "$device found" wait_more "dwc2d: mouse at address [0-9]+: $kind" 15 "$n"
    tr -d '\r' < "$log" | grep -aE "dwc2d: mouse at address" | tail -1 | sed 's/^/      /'
    # The input log switches on with the next rescan.
    sleep 2
    mouse_set "$name"

    n=$(count "$moved")
    $move1 > /dev/null
    check "$device moved" wait_more "$moved" 5 "$n"
    n=$(count "dwc2d: mouse: buttons \[true, false, false\]")
    monitor "mouse_button 1" > /dev/null
    check "$device left button down" wait_more "dwc2d: mouse: buttons \[true, false, false\]" 5 "$n"
    n=$(count "dwc2d: mouse: buttons \[false, false, false\]")
    monitor "mouse_button 0" > /dev/null
    check "$device left button up" wait_more "dwc2d: mouse: buttons \[false, false, false\]" 5 "$n"
    n=$(count "dwc2d: mouse: wheel")
    monitor "mouse_move 0 0 1" > /dev/null
    check "$device wheel" wait_more "dwc2d: mouse: wheel" 5 "$n"

    # The test command for the real Pi; QEMU's hub powers its ports
    # together, so dwc2d forgets and resets the device.
    n=$(count "dwc2d: mouse at address [0-9]+: $kind")
    devsh "echo mouse > /tmp/dwc2d-cycle" > /dev/null
    check "$device port switched off and on" wait_more "dwc2d: mouse at address [0-9]+: $kind" 15 "$n"
    sleep 2
    mouse_set "$name"
    n=$(count "$moved")
    $move2 > /dev/null
    check "$device moved after the port cycle" wait_more "$moved" 5 "$n"

    n=$(count "dwc2d: mouse removed")
    monitor "device_del pointer0" > /dev/null
    check "$device unplugged" wait_more "dwc2d: mouse removed" 10 "$n"
    sleep 2
}

pointer usb-mouse "QEMU USB Mouse" relative "dwc2d: mouse: move by -?[0-9]+ -?[0-9]+" \
    "rel_move 20 10" "rel_move -20 -10"
pointer usb-tablet "QEMU USB Tablet" absolute "dwc2d: mouse: move to [0-9]+ [0-9]+" \
    "abs_move 16384 8192" "abs_move 8192 4096"
# 16384 of 32767 is the middle: 32769 of 65535 for inputd.
check "tablet position scaled to 0..65535" bash -c "tr -d '\r' < '$log' | grep -aqE 'dwc2d: mouse: move to 3276[0-9] 1638[0-9]'"

check "devshd still answers" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'echo ready' 2>/dev/null | grep -q ready"
echo
tr -d '\r' < "$log" | grep -aE "dwc2d: mouse" | head -30 | sed 's/^/      /'
exit $failed
