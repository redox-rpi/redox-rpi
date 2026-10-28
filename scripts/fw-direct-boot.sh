#!/usr/bin/env bash
# Boot files for starting Redox straight from the Pi firmware, without U-Boot
# and the Redox bootloader (issue #15, tools/rpi-boot-stub/README.md).
#
# Takes the kernel, the initfs and the RedoxFS partition from a disk image
# built by scripts/build-image.sh and writes:
#   <out>/kernel8.img  rpi-boot-stub with the kernel built in
#   <out>/initramfs    the initfs, then the RedoxFS partition (live disk)
#
# With --qemu it boots them in QEMU raspi3b with -kernel, -initrd and -dtb,
# the way the Pi firmware loads kernel8.img and the initramfs; --test waits
# for the login prompt and exits with 0 when it appears.
#
# Needs: cargo with the aarch64-unknown-none target, aarch64-linux-gnu-objcopy,
# partx, fdtput (device-tree-compiler), qemu-system-aarch64 for --qemu.
#
# Usage:
#   scripts/fw-direct-boot.sh [--image <harddrive.img>] [--out <dir>]
#                             [--qemu] [--test] [--append <command line>]
#
#   --append  command line for QEMU (cmdline.txt on a Pi); NAME=value words
#             with an upper-case NAME become Redox environment variables

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img"
out="$ROOT/build/fw-direct"
qemu=no
test=no
append=""
while [ $# -gt 0 ]; do
    case "$1" in
        --image) image="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        --qemu) qemu=yes; shift ;;
        --test) qemu=yes; test=yes; shift ;;
        --append) append="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done
[ -f "$image" ] || { echo "$image missing, run scripts/build-image.sh" >&2; exit 1; }
mkdir -p "$out"
image="$(cd "$(dirname "$image")" && pwd)/$(basename "$image")"

tool="$ROOT/tools/redoxfs-file/target/release/redoxfs-file"
[ -x "$tool" ] || cargo build --release --manifest-path "$ROOT/tools/redoxfs-file/Cargo.toml"
"$tool" "$image" get usr/lib/boot/kernel "$out/kernel"
"$tool" "$image" get usr/lib/boot/initfs "$out/initfs"

stub="$ROOT/tools/rpi-boot-stub"
# Cargo reads the stub's .cargo/config.toml (target, linker script) only
# when it runs in the stub's directory.
(cd "$stub" && REDOX_KERNEL="$out/kernel" cargo build --release)
aarch64-linux-gnu-objcopy -O binary \
    "$stub/target/aarch64-unknown-none/release/rpi-boot-stub" "$out/kernel8.img"

# The RedoxFS partition is partition 3 of the image (BIOS, EFI, REDOX). The
# stub finds it at the first 4 KiB boundary after the initfs.
read -r start sectors < <(partx -g -o START,SECTORS -n 3 "$image")
cp "$out/initfs" "$out/initramfs"
truncate -s %4096 "$out/initramfs"
dd if="$image" bs=512 skip="$start" count="$sectors" status=none >> "$out/initramfs"
echo "kernel8.img: $(wc -c < "$out/kernel8.img") bytes, initramfs: $(($(wc -c < "$out/initramfs") >> 20)) MiB, in $out"

[ "$qemu" = yes ] || exit 0

# The mainline devicetree with what config.txt does on a real Pi, as in
# scripts/qemu-raspi3b.sh: console on the PL011, SD card on Arasan.
dtb="$ROOT/build/u-boot/dtb/bcm2837-rpi-3-b-plus.dtb"
[ -f "$dtb" ] || { echo "$dtb missing, run scripts/build-uboot.sh" >&2; exit 1; }
cp "$dtb" "$out/qemu.dtb"
fdtput -t s "$out/qemu.dtb" /chosen stdout-path "serial0:115200n8"
fdtput -t s "$out/qemu.dtb" /soc/mmc@7e202000 status disabled

qemu=(qemu-system-aarch64 -M raspi3b -smp 4 -m 1024
    -kernel "$out/kernel8.img" -initrd "$out/initramfs" -dtb "$out/qemu.dtb"
    -display none -usb -device usb-kbd)
[ -z "$append" ] || qemu+=(-append "$append")
if [ "$test" = no ]; then
    echo "Exit with Ctrl-A X."
    exec "${qemu[@]}" -serial mon:stdio -serial null
fi

log="$out/qemu-test.log"
rm -f "$log"
started=$SECONDS
"${qemu[@]}" -serial file:"$log" -serial null &
pid=$!
trap 'kill $pid 2>/dev/null || true' EXIT
for _ in $(seq 1 180); do
    if [ -f "$log" ] && tr -d '\000' < "$log" | grep -aq 'redox login:'; then
        echo "PASS: login prompt $((SECONDS - started)) s after the start of QEMU ($log)"
        exit 0
    fi
    kill -0 $pid 2>/dev/null || break
    sleep 1
done
echo "FAIL: no login prompt ($log)" >&2
tr -d '\000' < "$log" | grep -a 'rpi-boot-stub' >&2 || true
exit 1
