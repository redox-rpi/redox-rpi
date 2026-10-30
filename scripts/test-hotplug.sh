#!/usr/bin/env bash
# Check USB hotplug in dwc2d and the DHCP retry in QEMU, through QEMU's
# monitor. Two boots:
#
# 1. With the USB network card: unplug the USB keyboard, plug a new one in
#    and type on it at the login prompt; plug in a second network card and
#    switch the first one's hub port off and on through /tmp/dwc2d-cycle
#    (the test command for the real Pi, scripts/pi-hotplug-test.sh): the
#    network moves to the second card; then unplug the first card.
# 2. Without a network card: once the system is up, plug one in; dhcpd must
#    configure the network and devshd must answer.
#
# Needs an image built with scripts/build-image.sh (network boot). The host
# ports for devshd are 12323 and 12399, or HOTPLUG_PORTS="<first> <second>".
#
# Usage:
#   scripts/test-hotplug.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
socket="$ROOT/images/qemu-monitor.sock"
failed=0
read -r port1 port2 <<< "${HOTPLUG_PORTS:-12323 12399}"

devsh() {
    local port=$1
    shift
    python3 "$ROOT/scripts/devsh.py" 127.0.0.1 "$port" "$@" 2>/dev/null
}

# Send monitor commands; prints their output.
monitor() {
    python3 - "$socket" "$@" <<'PYEOF'
import socket, sys, time
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
    # Drop the echoed command and the prompt.
    print("\n".join(l for l in out.split("\n")[1:] if not l.startswith("(qemu)")))
PYEOF
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

# The QOM path of the first device of a type that QEMU added from the
# command line (they have no id).
qom_path() {
    local i path
    for i in 0 1 2 3 4 5; do
        path="/machine/peripheral-anon/device[$i]"
        if monitor "qom-get $path type" | grep -q "\"$1\""; then
            echo "$path"
            return
        fi
    done
}

# Wait up to $2 seconds for a line in the serial log.
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

wait_devsh() {
    local _
    for _ in $(seq "$2"); do
        devsh "$1" "echo ready" | grep -q ready && return 0
        sleep 3
    done
    return 1
}

# Each boot has its own serial log: $1 is its number.
boot() {
    log="$ROOT/images/qemu-hotplug-$1.log"
    shift
    rm -f "$socket" "$log"
    DEVSH_PORT=$port1 timeout 600 "$ROOT/scripts/qemu-raspi3b.sh" "$@" --monitor "$socket" --image "$image" \
        < /dev/null > "$log" 2>&1 &
    qemu=$!
    for _ in $(seq 20); do
        [ -S "$socket" ] && break
        sleep 1
    done
}

stop() {
    kill "$qemu" 2>/dev/null
    sleep 2
    pkill -P "$qemu" 2>/dev/null
    wait "$qemu" 2>/dev/null
}
trap 'stop' EXIT

echo "== boot 1: keyboard and network card hotplug"
boot 1 --usb-net
if ! wait_devsh $port1 60; then
    echo "FAIL: devshd did not answer"
    exit 1
fi

kbd=$(qom_path usb-kbd)
monitor "device_del $kbd" > /dev/null
check "keyboard unplugged" wait_log "dwc2d: keyboard removed" 10
monitor "device_add usb-kbd,id=hpkbd" > /dev/null
check "keyboard plugged in" wait_log "dwc2d: keyboard at address .*" 10
sleep 2
type_text $'root\n'
sleep 2
type_text $'password\n'
sleep 4
type_text $'touch /home/root/typed-after-hotplug\n'
sleep 3
typed=no
for _ in 1 2 3; do
    devsh $port1 "ls /home/root" | grep -q typed-after-hotplug && typed=yes && break
    sleep 3
done
check "typed on the new keyboard" test "$typed" = yes

net=$(qom_path usb-net)
monitor "netdev_add user,id=hpnet,hostfwd=tcp::$port2-:2323" \
    "device_add usb-net,netdev=hpnet,id=hpnet0" > /dev/null
check "second network card found" wait_log "dwc2d: CDC Ethernet at address 4" 10
check "network stays on the first card" wait_devsh $port1 5

# The test command for the real Pi: switch the active card's hub port off
# and on. QEMU's hub powers its ports together, so dwc2d forgets and resets
# the card; the network moves to the second card meanwhile. (Unplugging the
# active card with device_del would crash QEMU: its dwc2 asserts on a
# device removed while a channel still has a transfer pending to it.)
moves=$(tr -d '\r' < "$log" | grep -ac "dwc2d: network on the adapter")
devsh $port1 "echo network > /tmp/dwc2d-cycle" > /dev/null
check "network port switched off and on" wait_log "dwc2d: cycle: hub 1 port 2 power on" 15
check "network moved to the second card" \
    bash -c "sleep 2; [ \$(tr -d '\r' < '$log' | grep -ac 'dwc2d: network on the adapter') -gt $moves ]"
check "devshd through the second card" wait_devsh $port2 20
check "first card found again" \
    bash -c "[ \$(tr -d '\r' < '$log' | grep -ac 'hub 1 port 2: device connected') -ge 2 ]"

# Unplugging the card not in use is safe in QEMU.
monitor "device_del $net" > /dev/null
check "first card unplugged" wait_log "dwc2d: hub 1 port 2: device at address [0-9]+ removed" 10
check "devshd still through the second card" wait_devsh $port2 10
stop

echo "== boot 2: network card plugged in after boot"
boot 2
check "dhcpd keeps trying without a card" wait_log "dhcpd: .*trying again" 240
monitor "netdev_add user,id=hpnet,hostfwd=tcp::$port1-:2323" \
    "device_add usb-net,netdev=hpnet,id=hpnet0" > /dev/null
check "network card found" wait_log "dwc2d: CDC Ethernet at address" 10
check "dhcpd configured the network" wait_log "dhcpd: configured after" 30
check "devshd through the new card" wait_devsh $port1 20
stop

echo
tr -d '\r' < "$log" | grep -aE "dwc2d: (network|CDC|keyboard|hub .* removed)|dhcpd" | head -20
exit $failed
