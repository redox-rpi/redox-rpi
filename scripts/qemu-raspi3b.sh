#!/usr/bin/env bash
# Boot a Redox aarch64 image in QEMU's Raspberry Pi 3 machine (raspi3b).
#
# Works on macOS and Linux. Needs qemu-system-aarch64, curl, zstd and shasum
# (or sha256sum).
#
# Usage:
#   scripts/qemu-raspi3b.sh [--kernel <file>] [--image <file>]
#
#   --kernel <file>  replace usr/lib/boot/kernel in a copy of the image
#                    (uses tools/redoxfs-file)
#   --image <file>   boot this image instead of the latest daily server image
#
# The bootloader waits for a resolution choice; the script answers it on the
# serial console automatically. Exit QEMU with Ctrl-A X.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGES="$ROOT/images"
UBOOT_URL="https://gitlab.redox-os.org/Ivan/redox_firmware/-/raw/main/platform/raspberry_pi/rpi3/u-boot-rpi-3-b-plus.bin"
DAILY_URL="https://static.redox-os.org/img/aarch64"

kernel=""
image=""
while [ $# -gt 0 ]; do
    case "$1" in
        --kernel) kernel="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift 2 ;;
        --image) image="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done

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

# Confirm the bootloader's resolution menu, then hand the console to the user.
{ sleep 45; printf '\r'; cat; } | qemu-system-aarch64 \
    -M raspi3b -smp 4 -m 1024 \
    -kernel u-boot-rpi-3-b-plus.bin \
    -drive file=run.img,if=sd,format=raw \
    -serial stdio -display none -monitor none \
    -usb -device usb-kbd
