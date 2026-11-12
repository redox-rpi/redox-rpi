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
#   --card <file>    boot this SD card image as it is, e.g. one from
#                    scripts/make-sdcard.sh --local <img> --live --board pi4
#                    (U-Boot runs its boot.scr, the boot menu, the bootloader)
#   --test           no user input; exit 0 once every --expect pattern (default
#                    "redox login:") appeared on the serial console, 1 on a
#                    kernel panic or timeout. Log: images/qemu-raspi4b.log
#   --run <command>  after the login prompt, log in as root on the serial
#                    console and run this shell command (repeatable, implies
#                    --test); the output lands in the log
#   --timeout <s>    give up after this many seconds (default 300)
#   --shot <file>    at the end (after the login and any --run), save the
#                    screen (the framebuffer U-Boot handed over, #304) as a
#                    PPM, or PNG when the name ends in .png and ffmpeg is
#                    there; implies --test
#
# Interactive: the serial console is on stdin/stdout; exit with Ctrl-A X.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REDOX="$ROOT/upstream/redox"
IMAGES="$ROOT/images"
image="$REDOX/build/aarch64/minimal/harddrive.img"
test=no
card=
expects=()
run=()
shot=
timeout=300
while [ $# -gt 0 ]; do
    case "$1" in
        --image) image="$2"; shift 2 ;;
        --card) card="$2"; shift 2 ;;
        --test) test=yes; shift ;;
        --expect) expects+=("$2"); shift 2 ;;
        --run) run+=("$2"); test=yes; shift 2 ;;
        --timeout) timeout="$2"; shift 2 ;;
        --shot) shot="$2"; test=yes; shift 2 ;;
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

if [ -n "$card" ]; then
    cp "$card" run4.img
else
    cp "$image" run4.img
    efi_start=$(sfdisk -d run4.img | awk -F'[=,]' '/name="EFI"/ {gsub(/ /,"",$2); print $2}')
    mcopy -o -i "run4.img@@$((efi_start * 512))" "$live" ::/EFI/BOOT/BOOTAA64.EFI
fi
# QEMU's SD card must be a power of two in size.
size=$(wc -c < run4.img); sd=1
while [ "$sd" -lt "$size" ]; do sd=$((sd * 2)); done
truncate -s "$sd" run4.img

qemu=(qemu-system-aarch64 -M raspi4b -smp ${QEMU_SMP:-4} -m 2G
    -kernel "$uboot" -dtb qemu-4b.dtb
    -drive file=run4.img,if=sd,format=raw
    -display none)
monitor="$IMAGES/qemu-raspi4b.monitor"
if [ -n "$shot" ]; then
    rm -f "$monitor"
    qemu+=(-monitor "unix:$monitor,server,nowait")
else
    qemu+=(-monitor none)
fi

if [ "$test" = no ]; then
    exec "${qemu[@]}" -serial mon:stdio -serial null
fi

log="$IMAGES/qemu-raspi4b.log"
fifo="$IMAGES/qemu-raspi4b.in"
rm -f "$log" "$fifo"
mkfifo "$fifo"
"${qemu[@]}" -serial stdio -serial null < "$fifo" > "$log" 2>&1 &
pid=$!
exec 3> "$fifo"
trap 'kill $pid 2>/dev/null || true; rm -f "$fifo"' EXIT
start=$SECONDS
wait_for() {
    while ! grep -aqE "$1" "$log"; do
        if grep -aqE "KERNEL PANIC|panicked at .*kernel" "$log"; then
            echo "kernel panic, see $log" >&2; exit 1
        fi
        kill -0 $pid 2>/dev/null || { echo "QEMU exited, see $log" >&2; exit 1; }
        [ $((SECONDS - start)) -lt "$timeout" ] || { echo "timeout waiting for $1, see $log" >&2; exit 1; }
        sleep 1
    done
}
for re in "${expects[@]}"; do wait_for "$re"; done
echo "pass after $((SECONDS - start))s: ${expects[*]} (log $log)"
take_shot() {
    [ -n "$shot" ] || return 0
    sleep 3
    local ppm="${shot%.png}.ppm"
    case "$ppm" in /*) ;; *) ppm="$PWD/$ppm" ;; esac
    python3 -c 'import socket, sys, time
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); time.sleep(0.5)
s.sendall(("screendump " + sys.argv[2] + "\n").encode()); time.sleep(2)' "$monitor" "$ppm"
    [ -s "$ppm" ] || { echo "no screendump" >&2; return 1; }
    if [ "${shot%.png}" != "$shot" ] && command -v ffmpeg >/dev/null; then
        ffmpeg -loglevel error -y -i "$ppm" "$shot" && rm -f "$ppm" && ppm="$shot"
    fi
    echo "screen: $ppm"
}
[ ${#run[@]} -gt 0 ] || { take_shot; exit 0; }
sleep 2
printf 'root\r' >&3
wait_for "assword"
# login switches off the echo after the prompt; keys sent before are lost.
sleep 2
printf 'password\r' >&3
wait_for "root:"
line=""
for cmd in "${run[@]}"; do line="$line$cmd; "; done
printf '%secho RUN-DONE-$((40 + 2))\r' "$line" >&3
wait_for "RUN-DONE-42"
echo "run: done after $((SECONDS - start))s, output in $log"
take_shot
