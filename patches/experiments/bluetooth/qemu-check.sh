#!/usr/bin/env bash
# QEMU check for the bluetooth experiment (#54): QEMU's raspi3b has the
# mini UART but no Bluetooth chip, so fake-hci.py plays the chip on a TCP
# port that the mini UART connects to. QEMU's board is a 3B, so there is no
# bt_power file and `bt up` leaves the power alone. Needs the gpio
# experiment in the image too.
#
# Usage: patches/experiments/bluetooth/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
port="${QEMU_BT_PORT:-12399}"
hci_log="$ROOT/images/fake-hci.log"
mkdir -p "$ROOT/images"
rm -f "$hci_log"
python3 "$(dirname "$0")/fake-hci.py" "$port" "$hci_log" &
fake=$!
trap 'kill $fake 2>/dev/null' EXIT
for _ in $(seq 50); do
    grep -q "^listening" "$hci_log" 2>/dev/null && break
    sleep 0.1
done

"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 800 \
    --mini-uart "tcp:127.0.0.1:$port" \
    --run "echo P \$(cat /scheme/gpio/30/mode) \$(cat /scheme/gpio/33/mode) \$(cat /scheme/gpio/33/pull)" \
    --run "ls /scheme/btuart" \
    --run "bt up" \
    --run "bt info" \
    --run "bt scan 2" \
    --run "bt inquiry 2" \
    --run "bt cmd 1009" \
    --run "bt cmd fc99" \
    --run "echo 400000000 > /scheme/btuart/clock" \
    --run "bt diag" \
    --run "cat /scheme/btuart/clock" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
check() {
    if grep -qE "$2" <<< "$log"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
check "driver started" "bcm2835-btuartd: mini UART at 0x3f215000, 11[0-9]{4} baud"
check "pins 30 and 33 alt5, 33 pulled up" "^P alt5 alt5 up"
check "scheme files" "^baud[[:space:]]+clock[[:space:]]+data[[:space:]]+flow[[:space:]]+lpo[[:space:]]+status"
check "no bt_power on QEMU's 3B" "^power: no /scheme/rpi-firmware/bt_power"
check "ROM version, build 0" "^version: Bluetooth 4.1 \(HCI 7, LMP 7\), manufacturer 15 \(Broadcom\), subversion 6119, firmware build 0$"
check "firmware loaded" "^firmware: /usr/lib/firmware/brcm/BCM4345C0.hcd, 323 records"
check "firmware build after loading" "firmware build 344$"
check "up" "^up$"
check "address" "^address: 43:45:C0:00:1F:AC$"
check "name" "^name: BCM4345C0$"
check "LE scan finds the beacon" "^le 11:22:33:44:55:66 rssi -42 redox-beacon$"
check "LE scan count" "^scan: 2 devices$"
check "inquiry finds the PC" "^bredr FE:CA:EF:BE:ED:FE class 0a010c$"
check "inquiry count" "^inquiry: 1 devices$"
check "raw command" "^00 ac 1f 00 c0 45 43$"
check "unknown command status" "^01$"
check "diag: pins" "^gpio 32: mode alt5"
check "diag: lpo clock read" "^lpo: enabled [01], busy [01], source [0-9]+"
check "diag: line status" "^  after: cts_line [01], rts [01], tx_fifo 0"
check "diag: reset answered with each flow setting" "^answered with flow: auto, auto-low, none$"
check "diag: baud sweep answered (QEMU ignores the divider)" "^baud sweep answered for core MHz: 250, 275, 300, 325, 350, 375, 400, 450, 500$"
check "bt up takes the firmware's clock" "^uart: core clock [0-9]+ Hz set by firmware \(firmware (measures|reports) [0-9]+\)"
check "a clock set by hand stays" "^uart: core clock 400000000 Hz set by hand"
check "diag: core clock idle and under load" "^core clock under load: firmware reports [0-9]+, measures"
check "diag: reset at the driver's clock" "^reset at the driver's clock, flow auto: 04 0e 04 01 03 0c 00  <- Command Complete$"
check "the sweep gives the clock back to the firmware" "^firmware$"
if grep -q "^launch after 323 records$" "$hci_log"; then echo "PASS: fake saw the whole firmware"; else echo "FAIL: fake saw the whole firmware"; failed=1; fi
if grep -q "^stray byte" "$hci_log"; then echo "FAIL: stray bytes on the UART"; failed=1; else echo "PASS: no stray bytes"; fi
exit $failed
