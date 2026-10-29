#!/usr/bin/env bash
# Boot a Redox aarch64 image in QEMU's Raspberry Pi 3 machine (raspi3b).
#
# Works on macOS and Linux. Needs qemu-system-aarch64, curl, zstd and shasum
# (or sha256sum).
#
# Usage:
#   scripts/qemu-raspi3b.sh [--kernel <file>] [--image <file>] [--uboot ours|redox]
#                           [--test [--expect <regex>]... [--timeout <seconds>]]
#
#   --kernel <file>   replace usr/lib/boot/kernel in a copy of the image
#                     (uses tools/redoxfs-file)
#   --image <file>    boot this image instead of the latest daily server image
#   --uboot ours      our U-Boot from scripts/build-uboot.sh (default if built),
#                     with the mainline devicetree adjusted like the config.txt
#                     overlays on a real Pi (PL011 console, SD on Arasan)
#   --uboot redox     the prebuilt U-Boot 2022.07 from Redox
#   --usb-net         attach QEMU's USB network card (CDC Ethernet), with
#                     user networking: guest 10.0.2.15, host 10.0.2.2;
#                     host port 12323 forwards to devshd in the guest
#   --usb-host <vid:pid>
#                     pass this USB device of the host through to the guest,
#                     alone on the root port (no USB keyboard, no --usb-net):
#                     QEMU's hub is full speed, and dwc2d has no split bulk
#                     transfers. Needs read-write access to the device node
#                     (udev rule, see patches/experiments/rtl8152/README.md)
#   --mini-uart <spec>
#                     connect the mini UART (where the Pi's Bluetooth chip
#                     sits) to this QEMU character device instead of
#                     nothing, e.g. tcp:127.0.0.1:12399 for a fake
#                     controller (patches/experiments/bluetooth/)
#   --monitor <path>  QEMU monitor on this Unix socket, e.g. for typing on the
#                     USB keyboard with `sendkey`
#   --test            boot without user input and exit 0 once every --expect
#                     pattern appeared on the serial console (default:
#                     "redox login:"), 1 on a kernel panic or timeout.
#                     The console output is kept in images/qemu-test.log.
#   --expect <regex>  extended regex that must appear (repeatable)
#   --run <command>   after the expected patterns, log in as root on the
#                     serial console and run this shell command (repeatable,
#                     implies --test); the output lands in the log, and the
#                     script waits until all commands are done
#   --sd-size <MiB>   size of QEMU's SD card (a power of two, at least the
#                     image's size); above 2048 the card is SDHC
#   --timeout <s>     give up after this many seconds (default 900)
#
# The bootloader waits for a resolution choice: press Enter. In --test mode
# the script answers it automatically. Exit QEMU with Ctrl-A X.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGES="$ROOT/images"
UBOOT_URL="https://gitlab.redox-os.org/Ivan/redox_firmware/-/raw/main/platform/raspberry_pi/rpi3/u-boot-rpi-3-b-plus.bin"
DAILY_URL="https://static.redox-os.org/img/aarch64"
BOOT_MENU="Arrow keys and enter select mode"

kernel=""
image=""
test=no
expect=()
timeout=900
uboot=""
run=()
sd_size=""
usb_net=no
usb_host=""
monitor=(-monitor none)
mini_uart=""
while [ $# -gt 0 ]; do
    case "$1" in
        --kernel) kernel="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift 2 ;;
        --image) image="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift 2 ;;
        --test) test=yes; shift ;;
        --expect) expect+=("$2"); shift 2 ;;
        --timeout) timeout="$2"; shift 2 ;;
        --run) run+=("$2"); test=yes; shift 2 ;;
        --sd-size) sd_size="$2"; shift 2 ;;
        --uboot) uboot="$2"; shift 2 ;;
        --usb-net) usb_net=yes; shift ;;
        --usb-host) usb_host="$2"; shift 2 ;;
        --mini-uart) mini_uart="$2"; shift 2 ;;
        --monitor) monitor=(-monitor "unix:$(cd "$(dirname "$2")" && pwd)/$(basename "$2"),server,nowait"); shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done
[ ${#expect[@]} -gt 0 ] || expect=("redox login:")

mkdir -p "$IMAGES"
cd "$IMAGES"

sha256() {
    if command -v sha256sum >/dev/null; then sha256sum "$@"; else shasum -a 256 "$@"; fi
}

our_uboot="$ROOT/build/u-boot/u-boot.bin"
if [ -z "$uboot" ]; then
    if [ -f "$our_uboot" ]; then uboot=ours; else uboot=redox; fi
fi
case "$uboot" in
    ours)
        [ -f "$our_uboot" ] || { echo "$our_uboot missing, run scripts/build-uboot.sh" >&2; exit 1; }
        # Our U-Boot expects the devicetree from the Pi firmware. Use the
        # mainline one and apply what config.txt does on a real Pi:
        # disable-bt (console on the PL011) and mmc (SD card on Arasan).
        cp "$ROOT/build/u-boot/dtb/bcm2837-rpi-3-b-plus.dtb" qemu-3bp.dtb
        fdtput -t s qemu-3bp.dtb /chosen stdout-path "serial0:115200n8"
        fdtput -t s qemu-3bp.dtb /soc/mmc@7e202000 status disabled
        firmware=(-kernel "$our_uboot" -dtb qemu-3bp.dtb)
        # The second serial port is the mini UART, which stays unused.
        extra_serial=(-serial null)
        ;;
    redox)
        [ -f u-boot-rpi-3-b-plus.bin ] || curl -fsSLO "$UBOOT_URL"
        firmware=(-kernel u-boot-rpi-3-b-plus.bin)
        extra_serial=()
        ;;
    *) echo "unknown U-Boot: $uboot" >&2; exit 1 ;;
esac
echo "U-Boot: $uboot"
# The mini UART is the second serial port of the raspi machines.
if [ -n "$mini_uart" ]; then
    extra_serial=(-serial "$mini_uart")
fi

if [ -z "$image" ]; then
    name="$(curl -fsSL "$DAILY_URL/" | grep -oE 'redox_server_aarch64_[0-9-]+_[0-9]+_livedisk\.iso\.zst' | sort -u | tail -1)"
    if [ ! -f "$name" ]; then
        echo "downloading $name"
        curl -fsSLO "$DAILY_URL/$name"
        expected="$(curl -fsSL "$DAILY_URL/SHA256SUM" | grep "$name" | cut -d' ' -f1)"
        echo "$expected  $name" | sha256 -c -
    fi
    image="${name%.zst}"
    [ -f "$image" ] || zstd -dq "$name" -o "$image"
fi

# Never boot the downloaded image itself, QEMU and redoxfs-file write to it.
cp "$image" run.img
# QEMU's SD card must be a power of two in size (SD card images are not).
size=$(wc -c < run.img)
sd=1
while [ "$sd" -lt "$size" ]; do sd=$((sd * 2)); done
if [ -n "$sd_size" ]; then sd=$((sd_size * 1024 * 1024)); fi
[ "$sd" -eq "$size" ] || truncate -s "$sd" run.img

if [ -n "$kernel" ]; then
    tool="$ROOT/tools/redoxfs-file/target/release/redoxfs-file"
    [ -x "$tool" ] || cargo build --release --manifest-path "$ROOT/tools/redoxfs-file/Cargo.toml"
    "$tool" run.img put usr/lib/boot/kernel "$kernel"
fi

qemu=(qemu-system-aarch64
    -M raspi3b -smp 4 -m 1024
    "${firmware[@]}"
    -drive file=run.img,if=sd,format=raw
    -display none "${monitor[@]}"
    -usb)
if [ -n "$usb_host" ]; then
    if [ "$usb_net" = yes ]; then
        echo "--usb-host and --usb-net do not go together" >&2
        exit 1
    fi
    qemu+=(-device "usb-host,vendorid=0x${usb_host%%:*},productid=0x${usb_host##*:}")
else
    qemu+=(-device usb-kbd)
fi
if [ "$usb_net" = yes ]; then
    # Host port 12323 reaches devshd (port 2323) in the guest.
    qemu+=(-netdev user,id=usbnet,hostfwd=tcp::12323-:2323 -device usb-net,netdev=usbnet)
fi

if [ "$test" = no ]; then
    # QEMU gets the terminal directly, so it switches it to raw mode and keys
    # like Enter reach the guest unchanged. mon:stdio keeps Ctrl-A X working.
    echo "Press Enter at the bootloader's resolution menu. Exit with Ctrl-A X."
    "${qemu[@]}" -serial mon:stdio ${extra_serial[@]+"${extra_serial[@]}"}
    exit
fi

log="$IMAGES/qemu-test.log"
fifo="$IMAGES/qemu-test.in"
rm -f "$log" "$fifo"
mkfifo "$fifo"
"${qemu[@]}" -serial stdio ${extra_serial[@]+"${extra_serial[@]}"} < "$fifo" > "$log" 2>&1 &
pid=$!
exec 3> "$fifo"
trap 'kill $pid 2>/dev/null || true; rm -f "$fifo"' EXIT

menu_answered=no
start=$SECONDS
while true; do
    if [ "$menu_answered" = no ] && grep -aq "$BOOT_MENU" "$log"; then
        printf '\r' >&3
        menu_answered=yes
    fi
    missing=0
    for pattern in "${expect[@]}"; do
        grep -aEq "$pattern" "$log" || missing=$((missing + 1))
    done
    if [ $missing -eq 0 ]; then
        echo "PASS after $((SECONDS - start))s: all expected patterns seen"
        [ ${#run[@]} -gt 0 ] || exit 0
        break
    fi
    if grep -aq "KERNEL PANIC" "$log"; then
        grep -a -A4 "KERNEL PANIC" "$log" >&2
        echo "FAIL: kernel panic, see $log" >&2
        exit 1
    fi
    if ! kill -0 $pid 2>/dev/null; then
        echo "FAIL: QEMU exited, see $log" >&2
        exit 1
    fi
    if [ $((SECONDS - start)) -ge "$timeout" ]; then
        for pattern in "${expect[@]}"; do
            grep -aEq "$pattern" "$log" || echo "missing: $pattern" >&2
        done
        echo "FAIL: timeout after ${timeout}s, see $log" >&2
        exit 1
    fi
    sleep 2
done

# Log in on the serial console and run the --run commands as one line,
# followed by a marker that only appears in the output, not in the echo of
# the typed line.
wait_for() {
    while ! grep -aq "$1" "$log"; do
        if ! kill -0 $pid 2>/dev/null; then echo "FAIL: QEMU exited, see $log" >&2; exit 1; fi
        if [ $((SECONDS - start)) -ge "$timeout" ]; then echo "FAIL: timeout waiting for $1" >&2; exit 1; fi
        sleep 1
    done
}
sleep 2
printf 'root\r' >&3
wait_for "assword"
# login switches the terminal to no-echo after printing the prompt; keys sent
# before that are lost.
sleep 2
printf 'password\r' >&3
wait_for "root:"
line=""
for cmd in "${run[@]}"; do line="$line$cmd; "; done
printf '%secho RUN-DONE-$((40 + 2))\r' "$line" >&3
wait_for "RUN-DONE-42"
echo "RUN: done after $((SECONDS - start))s, output in $log"

