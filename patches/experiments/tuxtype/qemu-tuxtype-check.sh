#!/usr/bin/env bash
# QEMU check for Tux Typing (#529) and the German layout (keyboard-de):
# 1. types on QEMU's USB keyboard at the text console with the US key names
#    d e z y [ , which the German layout turns into "deyzü" (a file name);
# 2. starts the desktop and tuxtype with the German theme, prints its log,
#    whether it runs and the audio counter, and takes two screenshots
#    (images/tuxtype.ppm and images/tuxtype.ppm.2, the main menu);
# 3. opens Training, Basic Lessons and lesson 1 with the arrow keys and
#    Return, one screenshot each (images/tuxtype.ppm.3, .4 and .5).
# Build with the desktop experiments (../desktop/README.md), keyboard-de and tuxtype.
# Usage: qemu-tuxtype-check.sh [image]
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
mon=$ROOT/images/qemu-monitor-tuxtype.sock
log=$ROOT/images/qemu-test.log
rm -f "$mon" "$log" "$ROOT"/images/tuxtype.ppm*
H=/scheme/audiohw
(
    # Wait until the serial session runs (the console's login is up then).
    for _ in $(seq 400); do
        tr -d "\r" < "$log" 2>/dev/null | grep -aq "^TYPE-NOW" && break
        sleep 1
    done
    python3 - "$mon" <<'PY'
import socket, sys, time
m = socket.socket(socket.AF_UNIX); m.connect(sys.argv[1])
def keys(names):
    for k in names:
        m.sendall(f"sendkey {k}\n".encode()); time.sleep(0.2)
keys(["r", "o", "o", "t", "ret"]); time.sleep(2)
keys(list("password") + ["ret"]); time.sleep(4)
# touch deyzü: with the German layout, US z gives y, y gives z, [ gives ü
keys(["t", "o", "u", "c", "h", "spc", "d", "e", "z", "y", "bracket_left", "ret"])
PY
    for _ in $(seq 400); do
        [ "$(tr -d '\r' < "$log" | grep -ac "^loud_chunks")" -ge 2 ] && break
        sleep 1
    done
    python3 - "$mon" "$ROOT/images/tuxtype.ppm" <<'PY'
import socket, sys, time
m = socket.socket(socket.AF_UNIX); m.connect(sys.argv[1])
m.sendall(f"screendump {sys.argv[2]}\n".encode()); time.sleep(150)
m.sendall(f"screendump {sys.argv[2]}.2\n".encode()); time.sleep(3)
def keys(names):
    for k in names:
        m.sendall(f"sendkey {k}\n".encode()); time.sleep(0.5)
keys(["down", "down", "ret"]); time.sleep(15)
m.sendall(f"screendump {sys.argv[2]}.3\n".encode()); time.sleep(3)
keys(["ret"]); time.sleep(15)
m.sendall(f"screendump {sys.argv[2]}.4\n".encode()); time.sleep(3)
keys(["ret"]); time.sleep(15)
m.sendall(f"screendump {sys.argv[2]}.5\n".encode()); time.sleep(3)
PY
) &
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 900 --monitor "$mon" \
    --run "sleep 25; echo TYPE-NOW" \
    --run "sleep 25" \
    --run "ls /home/root/deyz* | od -An -c" \
    --run "desktop --yes" \
    --run "grep loud_chunks $H/stats" \
    --run "tuxtype --window --theme deutsch &> /tmp/tuxtype.log &" \
    --run "sleep 30" \
    --run "cat /tmp/tuxtype.log" \
    --run "ps | grep tuxtype" \
    --run "grep loud_chunks $H/stats" \
    --run "sleep 235" > /dev/null 2>&1
status=$?
out="$(tr -d '\r' < "$log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
check() {
    if grep -qaE "$2" <<< "$out"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
check "German layout: keys d e z y [ typed deyzü" "d +e +y +z +303"
check "SDL chose the Orbital video driver" "You are using the SDL orbital video driver"
check "tuxtype still runs" "/usr/bin/tuxtype"
grep -aE "^loud_chunks [0-9]+$" <<< "$out" | awk '{print $2}' | tr '\n' ' ' | sed 's/^/loud_chunks before and after: /'; echo
ls -la "$ROOT"/images/tuxtype.ppm* 2>/dev/null
exit $failed
