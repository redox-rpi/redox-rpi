#!/usr/bin/env bash
# USB hotplug on the real Pi without a person (#17): dwc2d switches the
# power of a hub port off and on (experiment usb-hotplug, /tmp/dwc2d-cycle),
# and this script checks in the serial log that the device went away and
# came back, and that devshd answers again over the network afterwards.
#
# Where the hub powers its ports only together (ganged), dwc2d forgets and
# resets the device instead; the log says which one happened ("disconnected
# after" or "still connected").
#
# Usage, on the build host:
#   [KEYBOARD=1] scripts/pi-hotplug-test.sh <pi address> [cycles] [serial log]
#
# Cycles the port of the network adapter in use `cycles` times (default 3);
# with KEYBOARD=1 then the USB keyboard's port once.

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
pi=$1
cycles=${2:-3}
serial=${3:-$HOME/redox-rpi/logs/serial.log}
failed=0

devsh() {
    python3 "$ROOT/scripts/devsh.py" "$pi" 2323 "$@" 2>/dev/null
}

# The serial log since byte $1, without escape sequences.
log_since() {
    tail -c +"$(($1 + 1))" "$serial" | tr -d '\r' | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g'
}

wait_devsh() {
    local _
    for _ in $(seq "$1"); do
        devsh "echo ready" | grep -q ready && return 0
        sleep 2
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

# Cycle one port: $1 is network or keyboard, $2 what must show up again.
cycle() {
    local what=$1 found=$2 start
    start=$(stat -c %s "$serial")
    local t0=$SECONDS
    # devsh runs over the network; with the network port cycled the session
    # may drop right after the write, which is fine.
    devsh "echo $what > /tmp/dwc2d-cycle" > /dev/null
    if [ "$what" = network ]; then
        check "$what: devshd answers again" wait_devsh 45
        echo "      back after $((SECONDS - t0)) s"
    else
        sleep 8
    fi
    local log
    log=$(log_since "$start")
    echo "$log" | grep -aE "dwc2d: (cycle|hub .* removed|network on|link|keyboard|LAN7800 at|RTL815x at|CDC Ethernet at)" | sed 's/^/      /'
    check "$what: port switched off" grep -aqE "dwc2d: cycle: hub [0-9]+ port [0-9]+ power off" <<< "$log"
    check "$what: device removed" grep -aqE "dwc2d: hub [0-9]+ port [0-9]+: device at address [0-9]+ removed" <<< "$log"
    check "$what: device found again" grep -aqE "$found" <<< "$log"
    if [ "$what" = network ]; then
        check "$what: network on the adapter again" grep -aqE "dwc2d: network on the adapter at address" <<< "$log"
    fi
}

if ! wait_devsh 5; then
    echo "FAIL: devshd on $pi does not answer"
    exit 1
fi
devsh "cat /etc/redox-rpi-release" | grep -a build | head -2

for i in $(seq "$cycles"); do
    echo "== network port, cycle $i of $cycles"
    cycle network "dwc2d: (LAN7800|RTL815x|CDC Ethernet) at address"
done

if [ "${KEYBOARD:-0}" = 1 ]; then
    echo "== keyboard port"
    cycle keyboard "dwc2d: keyboard at address"
fi

echo
grep -aE "dwc2d: .*hub at address .* power switching" "$serial" | tail -3 | tr -d '\r'
exit $failed
