#!/usr/bin/env bash
# Measure dwc2d's CPU time and the speed of USB transfers in QEMU: read and
# write a USB stick, send over TCP through QEMU's USB network card, and sit
# idle. dwc2d's CPU time comes from the ACTTIME column of
# /scheme/sys/context, before and after each step.
#
# Needs an image with USB sticks (experiment usb-storage). The host port for
# devshd is 12323, or MEASURE_PORT=<port>; the TCP sink listens on that port
# plus 1000 on the host.
#
# Usage:
#   scripts/measure-usb-cpu.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
socket="$ROOT/images/qemu-monitor-measure.sock"
log="$ROOT/images/qemu-measure.log"
stick="$ROOT/images/measure-stick.img"
run="$ROOT/images/measure-run.img"
port=${MEASURE_PORT:-12323}
sink=$((port + 1000))

devsh() {
    python3 "$ROOT/scripts/devsh.py" 127.0.0.1 "$port" "$@" 2>/dev/null | tr -d '\0\r' |
        sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g; s/\x1b\][^\x07]*\x07//g'
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
until_prompt()
for cmd in sys.argv[2:]:
    mon.sendall(cmd.encode() + b"\n")
    until_prompt()
PYEOF
}

# dwc2d's CPU time in seconds.
cpu() {
    devsh "grep dwc2d /scheme/sys/context" | grep -aoE '[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]{2}' | head -1 |
        awk -F: '{ printf "%.2f", $1 * 3600 + $2 * 60 + $3 }'
}

rate() {
    grep -aoE '[0-9.]+ [kMG]?B/s' | tail -1
}

report() {
    printf '%-22s %10s   dwc2d CPU %5.2f s\n' "$1" "$2" "$(echo "$4 - $3" | bc)"
}

rm -f "$stick" "$socket" "$log"
truncate -s 64M "$stick"
cp "$image" "$run"
DEVSH_PORT=$port timeout 900 "$ROOT/scripts/qemu-raspi3b.sh" --usb-net --monitor "$socket" \
    --image "$run" < /dev/null > "$log" 2>&1 &
qemu=$!
trap 'kill $qemu 2>/dev/null; sleep 1; pkill -P $qemu 2>/dev/null' EXIT

for _ in $(seq 100); do
    devsh "echo ready" | grep -q ready && break
    sleep 3
done
monitor "drive_add 0 if=none,id=stick,file=$stick,format=raw" "device_add usb-storage,drive=stick,id=stick0"
for _ in $(seq 30); do
    tr -d '\r' < "$log" | grep -aq "dwc2d: mass storage /scheme/disk" && break
    sleep 1
done
scheme=$(tr -d '\r' < "$log" | grep -aoE "/scheme/disk\.usb-dwc2\+[0-9.]+-scsi" | head -1)
echo "stick: $scheme"
sleep 5

a=$(cpu); sleep 30; b=$(cpu)
report "idle 30 s" "-" "$a" "$b"

a=$(cpu)
r=$(devsh "dd if=$scheme/0 of=/dev/null bs=65536 count=1024" | rate)
b=$(cpu)
report "stick read 64 MiB" "$r" "$a" "$b"

a=$(cpu)
r=$(devsh "dd if=/dev/zero of=$scheme/0 bs=65536 count=512 conv=notrunc" | rate)
b=$(cpu)
report "stick write 32 MiB" "$r" "$a" "$b"

# QEMU's user network: the guest reaches the host's loopback as 10.0.2.2.
# The sink closes after 16 MiB, which ends nc in the guest.
out=$(mktemp)
(timeout 120 nc -l 127.0.0.1 "$sink" 2>/dev/null || timeout 120 nc -l -p "$sink" 2>/dev/null) |
    head -c 16777216 | wc -c > "$out" &
sinkpid=$!
sleep 1
a=$(cpu)
start=$(date +%s.%N)
devsh "dd if=/dev/zero bs=65536 count=256 | nc 10.0.2.2:$sink" > /dev/null
end=$(date +%s.%N)
b=$(cpu)
wait $sinkpid 2>/dev/null
bytes=$(cat "$out"); rm -f "$out"
mbs=$(echo "scale=2; $bytes / ($end - $start) / 1000000" | bc)
report "tcp send 16 MiB" "$mbs MB/s" "$a" "$b"
[ "$bytes" = 16777216 ] || echo "      (sink got $bytes bytes)"

grep -ac panicked "$log" | sed 's/^/panics: /'
