#!/usr/bin/env bash
# QEMU check for the spi experiment (#42): QEMU's raspi3b emulates SPI0, and
# a W25Q32 flash sits on its bus. QEMU does not drive the chip select lines,
# so the flash stays selected and only commands of a fixed length work:
# JEDEC ID (9f, then ef 40 16 and three zeros) and the status register (05).
# Needs the gpio experiment in the image too.
#
# Usage: patches/experiments/spi/qemu-check.sh [image]

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
S=/scheme/spi/0
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 800 \
    --device w25q32,bus=spi \
    --run "cat /scheme/gpio/pins" \
    --run "ls $S" \
    --run "echo J1 \$(spi 9f 0 0 0 0 0 0)" \
    --run "echo J2 \$(spi 9f 0 0 0 0 0 0)" \
    --run "echo SR \$(spi 5 0)" \
    --run "spi -s 500000 -m 3" \
    --run "echo 4 > $S/mode" \
    --run "spi -m 0" \
    --run "spi -l 4096" \
    --run "spi -c 2 0" > /dev/null 2>&1
status=$?
log="$(tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
failed=0
check() {
    if grep -qE "$2" <<< "$log"; then echo "PASS: $1"; else echo "FAIL: $1"; failed=1; fi
}
[ $status -eq 0 ] || { echo "FAIL: QEMU run ($status)"; failed=1; }
check "driver started" "bcm2835-spid: bus 0 at 0x3f204000, [0-9]+ kHz"
for pin in 7 8 9 10 11; do check "pin $pin alt0" "^$pin alt0 "; done
check "bus directory" "^cs1$"
check "JEDEC ID" "^J1 [0-9a-f]{2} ef 40 16 00 00 00$"
check "JEDEC ID again" "^J2 [0-9a-f]{2} ef 40 16 00 00 00$"
check "status register" "^SR [0-9a-f]{2} 00$"
check "speed and mode set" "^speed (500000|4[0-9]{5}) Hz, mode 3$"
check "bad mode refused" "Invalid argument"
check "4096-byte transfer done" "^loopback: 4096 bytes at [0-9]+ Hz"
check "bad chip select" "bad chip select"
exit $failed
