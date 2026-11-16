#!/usr/bin/env bash
# Check several USB keyboards and pointers at once in dwc2d (base 0101,
# #450) in QEMU: two keyboards, a mouse and a tablet behind QEMU's hub, each
# on a host channel of its own; typing and pointer events with all of them
# plugged in; unplugging one frees its channel for the next device; a sixth
# input function is left out (five channels, 3 to 7).
#
# QEMU sends keys to one keyboard of its choice, so which keyboard typed is
# shown by unplugging: after the second keyboard goes, the first one types,
# and after the first one goes, a third one types on the freed channel.
#
# The host port for devshd is 12323, or MULTI_PORT=<port>.
#
# Usage:
#   scripts/test-usb-multi.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
socket="$ROOT/images/qemu-monitor-multi.sock"
qmp="$ROOT/images/qemu-qmp-multi.sock"
log="$ROOT/images/qemu-usb-multi.log"
port=${MULTI_PORT:-12323}
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
call("input-send-event", {"events": [
    {"type": "abs", "data": {"axis": "x", "value": int(sys.argv[2])}},
    {"type": "abs", "data": {"axis": "y", "value": int(sys.argv[3])}}]})
PYEOF
}

mouse_set() {
    local index
    index=$(monitor "info mice" | sed 's/\x1b\[[0-9;]*[a-zA-Z]//g' | grep -a "$1" | grep -oE "Mouse #[0-9]+" | grep -oE "[0-9]+" | head -1)
    [ -n "$index" ] && monitor "mouse_set $index" > /dev/null
}

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

count() {
    tr -d '\r' < "$log" | grep -acE "$1"
}

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

# Type `touch /home/root/$1` and wait for the file.
typed() {
    local _
    type_text "touch /home/root/$1"$'\n'
    for _ in 1 2 3 4 5; do
        devsh "ls /home/root" | grep -q "$1" && return 0
        sleep 2
    done
    return 1
}

# The QOM path of the keyboard qemu-raspi3b.sh adds without an id.
first_keyboard() {
    local path
    for path in $(monitor "qom-list /machine/peripheral-anon" | grep -oE "device\[[0-9]+\]"); do
        if monitor "qom-get /machine/peripheral-anon/$path type" | grep -q '"usb-kbd"'; then
            echo "/machine/peripheral-anon/$path"
            return
        fi
    done
}

rm -f "$socket" "$qmp" "$log"
DEVSH_PORT=$port timeout 900 "$ROOT/scripts/qemu-raspi3b.sh" --usb-net --monitor "$socket" --qmp "$qmp" \
    --device usb-kbd,id=kbd2 --device usb-mouse,id=mouse1 --device usb-tablet,id=tablet1 \
    --image "$image" < /dev/null > "$log" 2>&1 &
qemu=$!
trap 'kill $qemu 2>/dev/null; sleep 1; pkill -P $qemu 2>/dev/null' EXIT

for _ in $(seq 80); do
    devsh "echo ready" | grep -q ready && break
    sleep 3
done
devsh "touch /tmp/dwc2d-input-log" > /dev/null

echo "== at boot: two keyboards, a mouse, a tablet"
check "two keyboards" bash -c "[ \$(tr -d '\r' < '$log' | grep -acE 'dwc2d: keyboard at address [0-9]+ polls on channel') -eq 2 ]"
check "two pointers" bash -c "[ \$(tr -d '\r' < '$log' | grep -acE 'dwc2d: mouse at address [0-9]+ polls on channel') -eq 2 ]"
channels=$(tr -d '\r' < "$log" | grep -aoE "polls on channel [0-9]" | grep -oE "[0-9]$" | sort)
check "four different channels from 3 to 7" bash -c "[ \"\$(echo '$channels' | sort -u | grep -cE '^[3-7]$')\" -eq 4 ]"
tr -d '\r' < "$log" | grep -aE "polls on channel" | sed 's/^/      /'

# devshd starts right after the network (#217), before the console's login
# prompt: wait for the prompt. This also lets the input log switch on with
# the next rescan.
sleep 25
type_text $'root\n'
sleep 2
type_text $'password\n'
sleep 4
check "typing with both keyboards plugged in" typed multi-both

mouse_set "QEMU USB Mouse"
n=$(count "dwc2d: mouse: move by")
monitor "mouse_move 20 10" > /dev/null
check "mouse moves with the tablet plugged in" wait_more "dwc2d: mouse: move by" 5 "$n"
mouse_set "QEMU USB Tablet"
n=$(count "dwc2d: mouse: move to")
abs_move 16384 8192
check "tablet moves with the mouse plugged in" wait_more "dwc2d: mouse: move to" 5 "$n"

echo "== unplug the second keyboard"
n=$(count "dwc2d: keyboard removed")
monitor "device_del kbd2" > /dev/null
check "second keyboard removed" wait_more "dwc2d: keyboard removed" 10 "$n"
sleep 2
check "first keyboard still types" typed multi-first

echo "== a third keyboard, then unplug the first"
n=$(count "dwc2d: keyboard at address [0-9]+ polls on channel")
monitor "device_add usb-kbd,id=kbd3" > /dev/null
check "third keyboard on a freed channel" wait_more "dwc2d: keyboard at address [0-9]+ polls on channel" 15 "$n"
tr -d '\r' < "$log" | grep -aE "polls on channel" | tail -1 | sed 's/^/      /'
first=$(first_keyboard)
n=$(count "dwc2d: keyboard removed")
[ -n "$first" ] && monitor "device_del $first" > /dev/null
check "first keyboard removed" wait_more "dwc2d: keyboard removed" 10 "$n"
sleep 2
check "third keyboard types" typed multi-third

echo "== fill the pool"
# In use: keyboard 3, mouse, tablet. Two more fit, the sixth function does not.
monitor "device_add usb-mouse,id=mouse2" > /dev/null
sleep 6
monitor "device_add usb-kbd,id=kbd4" > /dev/null
sleep 6
n=$(count "left out: no free channel")
monitor "device_add usb-kbd,id=kbd5" > /dev/null
check "sixth function left out" wait_more "left out: no free channel" 15 "$n"
mouse_set "QEMU USB Mouse"
n=$(count "dwc2d: mouse: move by")
monitor "mouse_move -20 -10" > /dev/null
check "a mouse still moves with the pool full" wait_more "dwc2d: mouse: move by" 5 "$n"

check "devshd still answers" bash -c "python3 '$ROOT/scripts/devsh.py' 127.0.0.1 $port 'echo ready' 2>/dev/null | grep -q ready"
echo
tr -d '\r' < "$log" | grep -aE "dwc2d: (keyboard|mouse at|mouse removed|.*left out)" | head -30 | sed 's/^/      /'
exit $failed
