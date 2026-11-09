#!/usr/bin/env bash
# Boot a Redox aarch64 image in QEMU's Raspberry Pi 4 machine (raspi4b), the
# SoC of the Pi 400 (BCM2711, GIC-400). Work package #299, experiment pi400.
#
# QEMU's raspi4b has no PCIe (no USB, no keyboard) and no GENET (no
# network), so this checks the kernel, the interrupt controller, the timer,
# SMP and the serial console up to the login. The root filesystem runs from
# RAM: the script puts bootloader-live.efi in place of the image's UEFI
# bootloader, as on the Pi 400 test card (there is no EMMC2 driver yet).
#
# Needs: qemu-system-aarch64 (9.0 or newer), mtools, fdtput, and
# scripts/build-uboot.sh run first (U-Boot and the BCM2711 devicetree).
#
# Usage:
#   scripts/qemu-raspi4b.sh [--image <harddrive.img>] [--test [--expect <regex>]...]
#                           [--timeout <seconds>]
#
#   --image <file>   default: the network-boot image of scripts/build-image.sh
#   --test           no user input; exit 0 once every --expect pattern (default
#                    "redox login:") appeared on the serial console, 1 on a
#                    kernel panic or timeout. Log: images/qemu-raspi4b.log
#   --timeout <s>    give up after this many seconds (default 300)
#
# Interactive: the serial console is on stdin/stdout; exit with Ctrl-A X.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REDOX="$ROOT/upstream/redox"
IMAGES="$ROOT/images"
image="$REDOX/build/aarch64/minimal/harddrive.img"
test=no
expects=()
timeout=300
while [ $# -gt 0 ]; do
    case "$1" in
        --image) image="$2"; shift 2 ;;
        --test) test=yes; shift ;;
        --expect) expects+=("$2"); shift 2 ;;
        --timeout) timeout="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done
[ ${#expects[@]} -gt 0 ] || expects=("redox login:")

uboot="$ROOT/build/u-boot/u-boot.bin"
dtb="$ROOT/build/u-boot/dtb/bcm2711-rpi-4-b.dtb"
[ -f "$uboot" ] && [ -f "$dtb" ] || { echo "run scripts/build-uboot.sh first" >&2; exit 1; }
live="$(find "$REDOX/recipes/core/bootloader/target" -path '*stage/usr/lib/boot/bootloader-live.efi' | head -1)"
[ -n "$live" ] || { echo "bootloader-live.efi missing, run scripts/build-image.sh" >&2; exit 1; }

mkdir -p "$IMAGES"
cd "$IMAGES"
# The firmware would set these on a real Pi 400 (config.txt: enable_uart,
# dtoverlay=disable-bt): console on the PL011 (uart0).
cp "$dtb" qemu-4b.dtb
fdtput -t s qemu-4b.dtb /chosen stdout-path "serial0:115200n8"

cp "$image" run4.img
efi_start=$(sfdisk -d run4.img | awk -F'[=,]' '/name="EFI"/ {gsub(/ /,"",$2); print $2}')
mcopy -o -i "run4.img@@$((efi_start * 512))" "$live" ::/EFI/BOOT/BOOTAA64.EFI
# QEMU's SD card must be a power of two in size.
size=$(wc -c < run4.img); sd=1
while [ "$sd" -lt "$size" ]; do sd=$((sd * 2)); done
truncate -s "$sd" run4.img

qemu=(qemu-system-aarch64 -M raspi4b -smp ${QEMU_SMP:-4} -m 2G
    -kernel "$uboot" -dtb qemu-4b.dtb
    -drive file=run4.img,if=sd,format=raw
    -display none -monitor none)

if [ "$test" = no ]; then
    exec "${qemu[@]}" -serial mon:stdio -serial null
fi

log="$IMAGES/qemu-raspi4b.log"
: > "$log"
"${qemu[@]}" -serial "file:$log" -serial null < /dev/null &
pid=$!
trap 'kill $pid 2>/dev/null || true' EXIT
deadline=$((SECONDS + timeout))
while [ $SECONDS -lt $deadline ]; do
    if grep -aqE "KERNEL PANIC|panicked at .*kernel" "$log"; then
        echo "kernel panic, see $log" >&2; exit 1
    fi
    ok=yes
    for re in "${expects[@]}"; do grep -aqE "$re" "$log" || { ok=no; break; }; done
    if [ "$ok" = yes ]; then echo "pass: ${expects[*]} (log $log)"; exit 0; fi
    kill -0 $pid 2>/dev/null || { echo "QEMU exited, see $log" >&2; exit 1; }
    sleep 2
done
echo "timeout after ${timeout}s, see $log" >&2
exit 1
