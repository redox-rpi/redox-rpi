#!/usr/bin/env bash
# Boot a Redox aarch64 image in QEMU's Raspberry Pi 3 machine (raspi3b).
#
# Works on macOS and Linux. Needs qemu-system-aarch64, curl, zstd and shasum
# (or sha256sum).
#
# Usage:
#   scripts/qemu-raspi3b.sh [--kernel <file>] [--image <file>]
#                           [--test [--expect <regex>]... [--timeout <seconds>]]
#
#   --kernel <file>   replace usr/lib/boot/kernel in a copy of the image
#                     (uses tools/redoxfs-file)
#   --image <file>    boot this image instead of the latest daily server image
#   --test            boot without user input and exit 0 once every --expect
#                     pattern appeared on the serial console (default:
#                     "redox login:"), 1 on a kernel panic or timeout.
#                     The console output is kept in images/qemu-test.log.
#   --expect <regex>  extended regex that must appear (repeatable)
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
while [ $# -gt 0 ]; do
    case "$1" in
        --kernel) kernel="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift 2 ;;
        --image) image="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift 2 ;;
        --test) test=yes; shift ;;
        --expect) expect+=("$2"); shift 2 ;;
        --timeout) timeout="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done
[ ${#expect[@]} -gt 0 ] || expect=("redox login:")

mkdir -p "$IMAGES"
cd "$IMAGES"

sha256() {
    if command -v sha256sum >/dev/null; then sha256sum "$@"; else shasum -a 256 "$@"; fi
}

if [ ! -f u-boot-rpi-3-b-plus.bin ]; then
    curl -fsSLO "$UBOOT_URL"
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

if [ -n "$kernel" ]; then
    tool="$ROOT/tools/redoxfs-file/target/release/redoxfs-file"
    [ -x "$tool" ] || cargo build --release --manifest-path "$ROOT/tools/redoxfs-file/Cargo.toml"
    "$tool" run.img put usr/lib/boot/kernel "$kernel"
fi

qemu=(qemu-system-aarch64
    -M raspi3b -smp 4 -m 1024
    -kernel u-boot-rpi-3-b-plus.bin
    -drive file=run.img,if=sd,format=raw
    -display none -monitor none
    -usb -device usb-kbd)

if [ "$test" = no ]; then
    # QEMU gets the terminal directly, so it switches it to raw mode and keys
    # like Enter reach the guest unchanged. mon:stdio keeps Ctrl-A X working.
    echo "Press Enter at the bootloader's resolution menu. Exit with Ctrl-A X."
    "${qemu[@]}" -serial mon:stdio
    exit
fi

log="$IMAGES/qemu-test.log"
fifo="$IMAGES/qemu-test.in"
rm -f "$log" "$fifo"
mkfifo "$fifo"
"${qemu[@]}" -serial stdio < "$fifo" > "$log" 2>&1 &
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
        exit 0
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
