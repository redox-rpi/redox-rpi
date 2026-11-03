#!/usr/bin/env bash
# Create an SD card image for the Raspberry Pi 3, 3B+ and Zero 2 W: for network
# boot (default) or with Redox on the card (--local).
#
# Network boot:
# Partition 1 (FAT32, 63 MiB; --boot-size) holds the Raspberry Pi firmware, our U-Boot,
# config.txt and a boot script. Redox itself comes from the TFTP server on
# every boot (see scripts/netboot.sh), so a new build needs no new card.
# Partition 2 is a RedoxFS data partition that Redox mounts at /scheme/data, for
# everything that should survive a reboot. Redox mounts it by position
# (/scheme/disk.mmc/0p1, see config/raspi3bp.toml); it also has a fixed UUID
# (DATA_UUID) so tools can recognize it.
#
# Local boot (--local <harddrive.img>, built by scripts/build-image.sh --sdcard):
# partition 1 additionally holds the Redox bootloader as EFI/BOOT/BOOTAA64.EFI,
# which U-Boot starts; partition 2 is the Redox root filesystem from the image.
#
# Needs: curl, sfdisk, partx, mtools, cargo (builds tools/redoxfs-file).
# Run scripts/build-uboot.sh first, and for network boot
# scripts/netboot.sh --server <ip>.
#
# Usage:
#   scripts/make-sdcard.sh [--tftp-dir <dir>] [--dir <name>] [--out <file>]
#                          [--boot-size <MiB>] [--data-size <MiB>]
#   scripts/make-sdcard.sh --local <harddrive.img> [--small] [--out <file>]
#
#   --dir <name>       the directory of scripts/netboot.sh --dir whose boot.scr
#                      goes on the card (default redox; redox-zero2w for the
#                      Zero 2 W image)
#   --boot-size <MiB>  size of the boot area, partition 1 plus the first MiB
#                      (default 64). The files need about 12 MiB; below 48 the
#                      partition is FAT16, which the Pi firmware reads as well.
#                      With --data-size 0 this is the whole image: 32 fits a
#                      64 MB card.
#   --small            with --local: shrink the root RedoxFS to its contents
#                      (redoxfs-resize min) and make the root partition just
#                      as large. Needs the grow-root experiment in the image,
#                      which grows both to the whole card on the first boot.
#   --data-size <MiB>  size of the data partition (default 960, 0 for none).
#                      RedoxFS does not grow with the card, so pick the size
#                      you want to use.
#
# Also writes <out>.xz (a few MiB, the data partition is mostly zeros), which
# Raspberry Pi Imager ("Use custom") or SwiftFlash flashes directly. Or write
# <out> with dd.
#
# Partition 1 gets a `.uuid` file naming the image for SwiftFlash
# (github.com/zehrer/SwiftFlashCLI, docs/uuid-file.md): only the image block,
# never a device or flash UUID. VERSION in the environment sets its version
# (default: git describe).

set -euo pipefail

BOOT_MIB=64
DATA_UUID="7265646f-7872-7069-2d64-617461000001"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
. "$ROOT/scripts/rpi-firmware.sh"
tftp="$ROOT/build/tftp"
dir=redox
out="$ROOT/build/sdcard-netboot.img"
data_mib=960
image=
small=no
while [ $# -gt 0 ]; do
    case "$1" in
        --tftp-dir) tftp="$2"; shift 2 ;;
        --dir) dir="$2"; shift 2 ;;
        --boot-size) BOOT_MIB="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        --data-size) data_mib="$2"; shift 2 ;;
        --local) image="$2"; shift 2 ;;
        --small) small=yes; shift ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done

uboot="$ROOT/build/u-boot/u-boot.bin"
bootscr="$tftp/$dir/boot.scr"
[ -f "$uboot" ] || { echo "$uboot missing, run scripts/build-uboot.sh" >&2; exit 1; }
if [ -n "$image" ]; then
    [ -f "$image" ] || { echo "$image missing, run scripts/build-image.sh --sdcard" >&2; exit 1; }
    # The image's EFI partition (2) and RedoxFS partition (3), in sectors.
    read -r efi_start _ < <(partx -g -o START,SECTORS -n 2 "$image")
    read -r root_start root_sectors < <(partx -g -o START,SECTORS -n 3 "$image")
    data_mib=$(((root_sectors + 2047) / 2048))
    [ "$out" != "$ROOT/build/sdcard-netboot.img" ] || out="$ROOT/build/sdcard-redox.img"
    if [ "$small" = yes ]; then
        # The root RedoxFS in its own file, shrunk to what its files use.
        resize="$ROOT/upstream/redox/build/fstools/bin/redoxfs-resize"
        [ -x "$resize" ] || { echo "$resize missing, run scripts/build-image.sh first" >&2; exit 1; }
        root_fs="$out.root"
        dd if="$image" of="$root_fs" bs=512 skip="$root_start" count="$root_sectors" status=none
        "$resize" "$root_fs" min | grep -E "^(new size|redoxfs-resize)"
        # Its size, from the newest of the 256 headers at the start.
        fs_bytes=$(python3 - "$root_fs" <<'PY'
import struct, sys
ring = open(sys.argv[1], "rb").read(256 * 4096)
headers = [ring[i:i + 4096] for i in range(0, len(ring), 4096) if ring[i:i + 8] == b"RedoxFS\0"]
newest = max(headers, key=lambda h: struct.unpack_from("<Q", h, 40)[0])
print(struct.unpack_from("<Q", newest, 32)[0])
PY
)
        data_mib=$(((fs_bytes + (1 << 20) - 1) >> 20))
        truncate -s "${data_mib}M" "$root_fs"
    fi
else
    [ -f "$bootscr" ] || { echo "$bootscr missing, run scripts/netboot.sh --server <ip> --dir $dir" >&2; exit 1; }
fi

fetch_firmware "$ROOT"
cache="$firmware_cache"

tool="$ROOT/tools/redoxfs-file/target/release/redoxfs-file"
if [ -z "$image" ] && [ "$data_mib" -gt 0 ] && [ ! -x "$tool" ]; then
    cargo build --release --manifest-path "$ROOT/tools/redoxfs-file/Cargo.toml"
fi

# MBR, since the Pi boot ROM reads MBR only: FAT32 boot partition from 1 MiB
# to 64 MiB (FAT16 below 48 MiB, see --boot-size), then the data or root partition (type da, "non-FS data"; Redox
# finds its filesystems by content).
# FAT32 needs at least about 33 MiB; a smaller partition gets FAT16.
if [ "$BOOT_MIB" -ge 48 ]; then fat32=-F; fat_type=c; else fat32=; fat_type=e; fi
rm -f "$out"
truncate -s "$((BOOT_MIB + data_mib))M" "$out"
{
    echo "start=2048, size=$(((BOOT_MIB - 1) * 2048)), type=$fat_type, bootable"
    [ "$data_mib" -eq 0 ] || echo "start=$((BOOT_MIB * 2048)), type=da"
} | sfdisk -q "$out"

# Build the FAT partition in its own file, so it cannot grow past its size.
fat="$out.fat"
rm -f "$fat"
truncate -s "$((BOOT_MIB - 1))M" "$fat"
mformat -i "$fat" $fat32 -v REDOXBOOT ::

for file in "${FIRMWARE_FILES[@]}"; do
    [ "$(dirname "$file")" = . ] || mmd -i "$fat" -D s "::$(dirname "$file")" 2>/dev/null || true
    mcopy -i "$fat" "$cache/$file" "::$file"
done
mcopy -i "$fat" "$ROOT/config/config.txt" ::config.txt
if [ -n "$image" ]; then image_id=sdcard-redox; else image_id=sdcard-netboot; fi
cat > "$out.uuid" <<EOF
{
  "schemaVersion": 1,
  "image": {
    "id": "$image_id",
    "name": "Redox OS for Raspberry Pi (redox-rpi)",
    "version": "${VERSION:-$(git -C "$ROOT" describe --tags --always --dirty)}",
    "build": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "commit": "$(git -C "$ROOT" rev-parse --short HEAD)"
  }
}
EOF
mcopy -i "$fat" "$out.uuid" ::.uuid
rm -f "$out.uuid"
mcopy -i "$fat" "$uboot" ::u-boot.bin
if [ -n "$image" ]; then
    efi="$out.efi"
    rm -rf "$efi"
    mcopy -s -i "$image@@$((efi_start * 512))" ::EFI "$efi"
    mcopy -s -i "$fat" "$efi" ::EFI
    rm -rf "$efi"
else
    mcopy -i "$fat" "$bootscr" ::boot.scr
fi
echo "Boot partition:"
mdir -i "$fat" -/ ::
dd if="$fat" of="$out" bs=1M seek=1 conv=notrunc status=none
rm -f "$fat"

if [ -n "$image" ] && [ "$small" = yes ]; then
    dd if="$root_fs" of="$out" bs=1M seek="$BOOT_MIB" conv=notrunc status=none
    rm -f "$root_fs"
elif [ -n "$image" ]; then
    dd if="$image" of="$out" bs=512 skip="$root_start" count="$root_sectors" \
        seek="$((BOOT_MIB * 2048))" conv=notrunc status=none
elif [ "$data_mib" -gt 0 ]; then
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

# The build manifest goes with the image it describes (docs/versioning.md).
if [ -n "$image" ]; then
    manifest="$(grep -l "\"harddrive.img\" = \"$(sha256sum < "$image" | cut -d' ' -f1)\"" \
        "$ROOT"/build/manifests/*.toml 2>/dev/null | head -1 || true)"
    if [ -n "$manifest" ]; then
        cp "$manifest" "$out.manifest.toml"
    else
        echo "warning: no manifest in build/manifests/ matches $image" >&2
    fi
fi

xz -T0 -6 -k -f "$out"

echo "SD card image: $out"
echo "compressed:    $out.xz ($(du -h "$out.xz" | cut -f1))"
sfdisk -l "$out" | tail -n +8
