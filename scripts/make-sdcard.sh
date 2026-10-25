#!/usr/bin/env bash
# Create a network-boot SD card image for the Raspberry Pi 3, 3B+ and Zero 2 W.
#
# The card only holds the Raspberry Pi firmware, our U-Boot, config.txt and a
# boot script. Redox itself comes from the TFTP server on every boot (see
# scripts/netboot.sh), so a new build needs no new card.
#
# Needs: curl, sfdisk, mtools. Run scripts/build-uboot.sh and
# scripts/netboot.sh --server <ip> first.
#
# Usage:
#   scripts/make-sdcard.sh [--tftp-dir <dir>] [--out <file>]
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
SIZE_MIB=64

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
tftp="$ROOT/build/tftp"
out="$ROOT/build/sdcard-netboot.img"
while [ $# -gt 0 ]; do
    case "$1" in
        --tftp-dir) tftp="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
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

# One FAT32 partition starting at 1 MiB. The Pi boot ROM reads MBR only.
rm -f "$out"
truncate -s "${SIZE_MIB}M" "$out"
echo 'start=2048, type=c, bootable' | sfdisk -q "$out"
fat="$out@@1M"
mformat -i "$fat" -F -v REDOXBOOT ::

for file in "${FIRMWARE_FILES[@]}"; do
    [ "$(dirname "$file")" = . ] || mmd -i "$fat" -D s "::$(dirname "$file")" 2>/dev/null || true
    mcopy -i "$fat" "$cache/$file" "::$file"
done
mcopy -i "$fat" "$ROOT/config/config.txt" ::config.txt
mcopy -i "$fat" "$uboot" ::u-boot.bin
mcopy -i "$fat" "$bootscr" ::boot.scr

echo "SD card image: $out"
mdir -i "$fat" -/ ::
