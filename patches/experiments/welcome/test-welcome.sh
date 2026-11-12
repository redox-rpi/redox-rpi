#!/usr/bin/env bash
# Check the first-boot welcome app (#24) in QEMU: answer its questions on
# the serial console, log in and show the files it wrote, then boot the
# same card again and check that it goes straight to the login prompt.
# Needs an image built with --experiment welcome. Uses no network port.
#
# Usage (on the build host):
#   patches/experiments/welcome/test-welcome.sh [image]

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
log="$ROOT/images/qemu-test.log"
fifo="$ROOT/images/qemu-test.in"
out="$ROOT/images/qemu-welcome.out"
second="$ROOT/images/qemu-welcome-2.img"
mkdir -p "$ROOT/images"
rm -f "$log"

"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 600 \
    --run "cat /etc/keymap" --run "cat /etc/hostname" --run "echo" \
    --run "cat /etc/net/wifi" --run "ls -l /etc/net/wifi" --run "cat /etc/welcome.done" \
    --run "ls /tmp" \
    < /dev/null > "$out" 2>&1 &
qemu=$!
trap 'kill $qemu 2>/dev/null || true' EXIT

# Waits for $1 on the serial console, then types $2 there
answer() {
    for _ in $(seq 300); do
        grep -aqF "$1" "$log" 2>/dev/null && break
        sleep 2
    done
    grep -aqF "$1" "$log" || { echo "FAIL: no \"$1\" on the console"; exit 1; }
    sleep 1
    printf '%s\r' "$2" > "$fifo"
}

answer "Press Enter to set" ""
answer "Keyboard layout [" "gb"
answer "Name of this computer" "pi-test"
answer "root password" ""
answer "Wi-Fi network name" "TestNet"
answer "Passphrase" "secret123"

wait $qemu || { cat "$out"; echo "FAIL: QEMU run, see $log"; exit 1; }
cp "$log" "$ROOT/images/qemu-welcome-1.log"
tr -d '\r' < "$log" | sed -n '/Welcome to Redox OS on/,$p' | grep -av '^\s*$' | tail -40

fail=0
check() {
    tr -d '\r' < "$log" | grep -aqE "$1" || { echo "FAIL: missing $1"; fail=1; }
}
check '^gb$'
check '^pi-test'
check '^ssid=TestNet$'
check '^psk=secret123$'
check '^-rw------- .*/etc/net/wifi'
check 'First-boot setup done'
if tr -d '\r' < "$log" | grep -aq 'Passphrase.*secret123'; then
    echo "FAIL: passphrase echoed"; fail=1
fi

# Second boot of the same card: straight to the login prompt
cp "$ROOT/images/run.img" "$second"
rm -f "$log"
"$ROOT/scripts/qemu-raspi3b.sh" --image "$second" --timeout 600 --run "cat /etc/hostname" \
    < /dev/null > "$out" 2>&1 || { cat "$out"; echo "FAIL: second boot"; exit 1; }
if grep -aq "Welcome to Redox OS on the Raspberry Pi" "$log"; then
    echo "FAIL: the welcome app ran again"; fail=1
fi
check '^pi-test'
cp "$log" "$ROOT/images/qemu-welcome-2.log"
rm -f "$second"

[ $fail = 0 ] && echo "PASS: welcome app wrote the files and did not run again"
exit $fail
