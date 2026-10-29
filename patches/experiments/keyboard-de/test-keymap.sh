#!/usr/bin/env bash
# Check the German keyboard layout (#36) in QEMU: the image switches inputd to
# `de` at boot (config.toml); type on QEMU's USB keyboard at the console's
# login prompt, and compare the file the typed command wrote. Needs an image built
# with --experiment keyboard-de. Uses no network port, so it can run while
# other QEMU tests use port 12323.
#
# Usage (on the build host):
#   patches/experiments/keyboard-de/test-keymap.sh [image]

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
socket="$ROOT/images/qemu-keymap-monitor.sock"
log="$ROOT/images/qemu-test.log"
out="$ROOT/images/qemu-keymap.out"
mkdir -p "$ROOT/images"
rm -f "$socket" "$log"

# The serial console only waits while the USB keyboard types, then shows the
# file.
"$ROOT/scripts/qemu-raspi3b.sh" --monitor "$socket" --image "$image" --timeout 600 \
    --run "sleep 150" --run "cat /home/root/de.txt" \
    < /dev/null > "$out" 2>&1 &
qemu=$!
trap 'kill $qemu 2>/dev/null || true' EXIT

# The serial console logs in before the boot ends; type once the rootfs
# services (devshd comes late, after the keymap unit) are up.
for _ in $(seq 300); do
    grep -aq "sleep 150" "$log" 2>/dev/null && grep -aq "devshd" "$log" && break
    sleep 2
done
sleep 10

# QEMU key names, i.e. the keys of a US keyboard.
sendkeys() {
    python3 - "$socket" "$@" <<'PYEOF'
import socket, sys, time
mon = socket.socket(socket.AF_UNIX)
mon.connect(sys.argv[1])
for key in sys.argv[2:]:
    mon.sendall(f"sendkey {key}\n".encode())
    time.sleep(0.15)
time.sleep(1)
PYEOF
}

sendkeys r o o t ret
sleep 2
sendkeys p a s s w o r d ret
sleep 4
# cat > /home/root/de.txt, then the characters into cat (ion would parse
# them), then Ctrl-D
sendkeys c a t spc shift-less spc shift-7 h o m e shift-7 r o o t shift-7 d e dot t x t ret
sleep 1
# zyß@{[]}\~|#öäü€"§'<>
sendkeys y z minus alt_r-q alt_r-7 alt_r-8 alt_r-9 alt_r-0 alt_r-minus alt_r-bracket_right alt_r-less \
    backslash semicolon apostrophe bracket_left alt_r-e shift-2 shift-3 shift-backslash less shift-less ret ctrl-d
sleep 2
# What the screen shows, for a failure.
python3 - "$socket" "$ROOT/images/qemu-keymap.ppm" <<'PYEOF'
import socket, sys, time
mon = socket.socket(socket.AF_UNIX)
mon.connect(sys.argv[1])
mon.sendall(f"screendump {sys.argv[2]}\n".encode())
time.sleep(2)
PYEOF

wait $qemu || { cat "$out"; echo "FAIL: QEMU run, see $log"; exit 1; }
expected='zyß@{[]}\~|#öäü€"§'"'"'<>'
if tr -d '\r' < "$log" | grep -aqxF "$expected"; then
    echo "PASS: German layout typed $expected"
else
    echo "FAIL: expected the line $expected in $log; cat printed:"
    tr -d '\r' < "$log" | grep -a -A3 "cat /home/root/de.txt" | tail -4
    echo "screen: $ROOT/images/qemu-keymap.ppm"
    exit 1
fi
