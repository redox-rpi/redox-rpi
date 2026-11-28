#!/usr/bin/env bash
# Boot files for starting Redox straight from the Pi firmware, without U-Boot
# and the Redox bootloader (issue #15, tools/rpi-boot-stub/README.md).
#
# Takes the kernel, the initfs and the RedoxFS partition from a disk image
# built by scripts/build-image.sh and writes:
#   <out>/kernel8.img  rpi-boot-stub with the kernel built in
#   <out>/initramfs    the initfs, then the RedoxFS partition (live disk)
#   <out>/harddrive.img a copy of the image whose banner says "direct"
#
# With --card-root the initramfs is the initfs alone and the root filesystem
# stays on the card (scripts/make-sdcard.sh --direct --card-root puts it
# there): bcm2835-sdhostd and redoxfs in the initfs mount it by its UUID,
# which goes into <out>/cmdline.txt as REDOXFS_UUID. The 222 MiB live disk
# then no longer takes half the RAM of a Zero 2 W (#786).
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
#                             [--boot-only] [--card-root]
#
#   --append  command line for QEMU (cmdline.txt on a Pi); NAME=value words
#             with an upper-case NAME become Redox environment variables
#   --boot-only  build nothing: boot the kernel8.img and initramfs already in
#             --out (scripts/console-image.sh --test); implies --qemu
#   --card-root  root filesystem on the card instead of in the initramfs;
#             with --qemu the image's RedoxFS partition is the SD card

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img"
out="$ROOT/build/fw-direct"
qemu=no
test=no
append=""
boot_only=no
card_root=no
while [ $# -gt 0 ]; do
    case "$1" in
        --image) image="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        --qemu) qemu=yes; shift ;;
        --test) qemu=yes; test=yes; shift ;;
        --append) append="$2"; shift 2 ;;
        --boot-only) boot_only=yes; qemu=yes; shift ;;
        --card-root) card_root=yes; shift ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done
mkdir -p "$out"
out="$(cd "$out" && pwd)"
if [ "$boot_only" = no ]; then
[ -f "$image" ] || { echo "$image missing, run scripts/build-image.sh" >&2; exit 1; }
image="$(cd "$(dirname "$image")" && pwd)/$(basename "$image")"

tool="$ROOT/tools/redoxfs-file/target/release/redoxfs-file"
[ -x "$tool" ] || cargo build --release --manifest-path "$ROOT/tools/redoxfs-file/Cargo.toml"
"$tool" "$image" get usr/lib/boot/kernel "$out/kernel"
"$tool" "$image" get usr/lib/boot/initfs "$out/initfs"

# build-image.sh names a network-boot image "netboot" ("release-netboot" for the
# release variant without devshd, "zero2w" for the
# Zero 2 W image) in the login banner and the release files. The live disk
# of this boot comes from the card: call it "direct" there, in a copy of the
# image. The build ID still tells the two apart.
[ "$image" = "$out/harddrive.img" ] || cp "$image" "$out/harddrive.img"
image="$out/harddrive.img"
for file in etc/issue etc/redox-rpi-release etc/redox-rpi-manifest; do
    "$tool" "$image" get "$file" "$out/release-file"
    sed -i -E -e 's/^(redox-rpi .*) \((netboot|release-netboot|zero2w), /\1 (direct, /' \
        -e 's/^IMAGE="(netboot|release-netboot|zero2w)"$/IMAGE="direct"/' \
        -e 's/^image = "(netboot|release-netboot|zero2w)"$/image = "direct"/' "$out/release-file"
    "$tool" "$image" put "$file" "$out/release-file"
done
rm "$out/release-file"

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
rm -f "$out/cmdline.txt"
if [ "$card_root" = yes ]; then
    # The RedoxFS header: signature "RedoxFS\0", version, then the UUID.
    uuid="$(dd if="$image" bs=512 skip="$start" count=1 status=none | od -An -tx1 -j16 -N16 | tr -d ' \n')"
    [ "${#uuid}" = 32 ] || { echo "no RedoxFS UUID in partition 3 of $image" >&2; exit 1; }
    uuid="${uuid:0:8}-${uuid:8:4}-${uuid:12:4}-${uuid:16:4}-${uuid:20:12}"
    # The words the stub passes into the environment, as the bootloader does
    # for a root filesystem on a disk.
    echo "REDOXFS_UUID=$uuid REDOXFS_BLOCK=0000000000000000" > "$out/cmdline.txt"
else
    truncate -s %4096 "$out/initramfs"
    dd if="$image" bs=512 skip="$start" count="$sectors" status=none >> "$out/initramfs"
fi
echo "kernel8.img: $(wc -c < "$out/kernel8.img") bytes, initramfs: $(($(wc -c < "$out/initramfs") >> 20)) MiB, in $out"
[ ! -f "$out/cmdline.txt" ] || echo "cmdline.txt: $(cat "$out/cmdline.txt")"
fi

[ "$qemu" = yes ] || exit 0

# The mainline devicetree with what config.txt does on a real Pi, as in
# scripts/qemu-raspi3b.sh: console on the PL011, SD card on Arasan.
dtb="$ROOT/build/u-boot/dtb/bcm2837-rpi-3-b-plus.dtb"
[ -f "$dtb" ] || { echo "$dtb missing, run scripts/build-uboot.sh" >&2; exit 1; }
cp "$dtb" "$out/qemu.dtb"
fdtput -t s "$out/qemu.dtb" /chosen stdout-path "serial0:115200n8"
# With the root on the card, sdhost stays: the initfs starts bcm2835-sdhostd,
# and QEMU moves the card to sdhost when the driver sets GPIO 48-53 to ALT0.
[ -f "$out/cmdline.txt" ] || fdtput -t s "$out/qemu.dtb" /soc/mmc@7e202000 status disabled

qemu=(qemu-system-aarch64 -M raspi3b -smp 4 -m 1024
    -kernel "$out/kernel8.img" -initrd "$out/initramfs" -dtb "$out/qemu.dtb"
    -display none -usb -device usb-kbd)
if [ -f "$out/cmdline.txt" ]; then
    # The card: the image's RedoxFS partition alone is enough, redoxfs finds
    # it by its UUID. QEMU wants a power-of-two size for an SD card.
    [ -f "$image" ] || image="$out/harddrive.img"
    read -r start sectors < <(partx -g -o START,SECTORS -n 3 "$image")
    dd if="$image" of="$out/card.img" bs=512 skip="$start" count="$sectors" status=none
    size=$(stat -c %s "$out/card.img"); card=1048576
    while [ "$card" -lt "$size" ]; do card=$((card * 2)); done
    truncate -s "$card" "$out/card.img"
    qemu+=(-drive file="$out/card.img",if=sd,format=raw)
    append="$(cat "$out/cmdline.txt")${append:+ $append}"
fi
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
        tr -d '\000' < "$log" | grep -a '^redox-rpi ' | tail -1
        exit 0
    fi
    kill -0 $pid 2>/dev/null || break
    sleep 1
done
echo "FAIL: no login prompt ($log)" >&2
tr -d '\000' < "$log" | grep -a 'rpi-boot-stub' >&2 || true
exit 1
