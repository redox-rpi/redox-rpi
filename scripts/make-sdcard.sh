#!/usr/bin/env bash
# Create a network-boot SD card image for the Raspberry Pi 3, 3B+ and Zero 2 W.
#
# Partition 1 (FAT32, 63 MiB) holds the Raspberry Pi firmware, our U-Boot,
# config.txt and a boot script. Redox itself comes from the TFTP server on
# every boot (see scripts/netboot.sh), so a new build needs no new card.
# Partition 2 is a RedoxFS data partition that Redox mounts at /scheme/data, for
# everything that should survive a reboot. Redox mounts it by position
# (/scheme/disk.mmc/0p1, see config/raspi3bp.toml); it also has a fixed UUID
# (DATA_UUID) so tools can recognize it.
#
# Needs: curl, sfdisk, mtools, cargo (builds tools/redoxfs-file).
# Run scripts/build-uboot.sh and scripts/netboot.sh --server <ip> first.
#
# Usage:
#   scripts/make-sdcard.sh [--tftp-dir <dir>] [--out <file>] [--data-size <MiB>]
#
#   --data-size <MiB>  size of the data partition (default 960, 0 for none).
#                      RedoxFS does not grow with the card, so pick the size
#                      you want to use.
#
# Write the result with Raspberry Pi Imager ("Use custom") or dd.

set -euo pipefail

# Raspberry Pi firmware release, see github.com/raspberrypi/firmware/tags
FIRMWARE_TAG="1.20260915"
FIRMWARE_URL="https://raw.githubusercontent.com/raspberrypi/firmware/$FIRMWARE_TAG/boot"
FIRMWARE_FILES=(
    bootcode.bin start.elf fixup.dat start_cd.elf fixup_cd.dat LICENCE.broadcom
    bcm2710-rpi-3-b.dtb bcm2710-rpi-3-b-plus.dtb bcm2710-rpi-zero-2-w.dtb
    overlays/disable-bt.dtbo overlays/mmc.dtbo
)
BOOT_MIB=64
DATA_UUID="7265646f-7872-7069-2d64-617461000001"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
tftp="$ROOT/build/tftp"
out="$ROOT/build/sdcard-netboot.img"
data_mib=960
while [ $# -gt 0 ]; do
    case "$1" in
        --tftp-dir) tftp="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        --data-size) data_mib="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done

uboot="$ROOT/build/u-boot/u-boot.bin"
bootscr="$tftp/redox/boot.scr"
[ -f "$uboot" ] || { echo "$uboot missing, run scripts/build-uboot.sh" >&2; exit 1; }
[ -f "$bootscr" ] || { echo "$bootscr missing, run scripts/netboot.sh --server <ip>" >&2; exit 1; }

cache="$ROOT/build/firmware/$FIRMWARE_TAG"
for file in "${FIRMWARE_FILES[@]}"; do
    if [ ! -f "$cache/$file" ]; then
        mkdir -p "$(dirname "$cache/$file")"
        curl -fsSL -o "$cache/$file" "$FIRMWARE_URL/$file"
    fi
done

tool="$ROOT/tools/redoxfs-file/target/release/redoxfs-file"
if [ "$data_mib" -gt 0 ] && [ ! -x "$tool" ]; then
    cargo build --release --manifest-path "$ROOT/tools/redoxfs-file/Cargo.toml"
fi

# MBR, since the Pi boot ROM reads MBR only: FAT32 boot partition from 1 MiB
# to 64 MiB, then the data partition (type da, "non-FS data").
rm -f "$out"
truncate -s "$((BOOT_MIB + data_mib))M" "$out"
{
    echo "start=2048, size=$(((BOOT_MIB - 1) * 2048)), type=c, bootable"
    [ "$data_mib" -eq 0 ] || echo "start=$((BOOT_MIB * 2048)), type=da"
} | sfdisk -q "$out"

# Build the FAT partition in its own file, so it cannot grow past its size.
fat="$out.fat"
rm -f "$fat"
truncate -s "$((BOOT_MIB - 1))M" "$fat"
mformat -i "$fat" -F -v REDOXBOOT ::

for file in "${FIRMWARE_FILES[@]}"; do
    [ "$(dirname "$file")" = . ] || mmd -i "$fat" -D s "::$(dirname "$file")" 2>/dev/null || true
    mcopy -i "$fat" "$cache/$file" "::$file"
done
mcopy -i "$fat" "$ROOT/config/config.txt" ::config.txt
mcopy -i "$fat" "$uboot" ::u-boot.bin
mcopy -i "$fat" "$bootscr" ::boot.scr
echo "Boot partition:"
mdir -i "$fat" -/ ::
dd if="$fat" of="$out" bs=1M seek=1 conv=notrunc status=none
rm -f "$fat"

if [ "$data_mib" -gt 0 ]; then
    data="$out.data"
    rm -f "$data"
    truncate -s "${data_mib}M" "$data"
    printf '%s\n' \
        "This is the data partition of a redox-rpi SD card (https://github.com/redox-rpi)." \
        "Redox mounts it at /scheme/data. Everything stored here survives a reboot." \
        > "$out.readme"
    "$tool" "$data" mkfs "$DATA_UUID" "$out.readme" >/dev/null
    rm -f "$out.readme"
    dd if="$data" of="$out" bs=1M seek="$BOOT_MIB" conv=notrunc status=none
    rm -f "$data"
fi

echo "SD card image: $out"
sfdisk -l "$out" | tail -n +8
