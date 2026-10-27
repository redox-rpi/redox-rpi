#!/usr/bin/env bash
# Prepare a TFTP directory for booting Redox over the network with U-Boot.
#
# Copies the disk image (gzip-compressed) and the live bootloader into
# <tftp-dir>/<dir>/ and compiles config/netboot.cmd into netboot.scr there,
# plus boot.scr: put that on the SD card's FAT partition and U-Boot
# network-boots on its own.
#
# Run after scripts/build-image.sh and scripts/build-uboot.sh.
#
# Usage:
#   scripts/netboot.sh --server <ip> [--board rpi|zero2w|qemu] [--dir <name>]
#                      [--tftp-dir <dir>] [--image <file>]
#
#   --server <ip>     address of the TFTP server as the board sees it
#                     (QEMU user networking: 10.0.2.2)
#   --board           rpi (default, Pi 3 and 3B+), zero2w (Pi Zero 2 W, 512 MB
#                     RAM, image at most 128 MiB) or qemu; selects the RAM
#                     addresses
#   --dir <name>      directory below the TFTP root, so boards with different
#                     images share one server (default: redox, for zero2w
#                     redox-zero2w). The SD card's boot.scr names it.
#   --tftp-dir <dir>  TFTP root (default: build/tftp)
#   --image <file>    disk image (default: the image from build-image.sh, for
#                     zero2w the one from build-image.sh --zero2w)
#   --tftp-only       load everything over TFTP, not HTTP (default for zero2w:
#                     U-Boot's r8152 driver fails to receive the HTTP transfer)
#   --http            use HTTP, with TFTP as fallback (default for rpi, qemu)
#   --tftp-blocksize <bytes>
#                     TFTP block size (default: U-Boot's 1468, for zero2w
#                     512: U-Boot's r8152 driver receives into a 2048-byte
#                     buffer, and two full-size frames that the adapter
#                     packs into one USB transfer overflow it)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REDOX="$ROOT/upstream/redox"

server=""
board=rpi
tftp="$ROOT/build/tftp"
dir=""
http=""
blocksize=""
image=""
while [ $# -gt 0 ]; do
    case "$1" in
        --server) server="$2"; shift 2 ;;
        --board) board="$2"; shift 2 ;;
        --dir) dir="$2"; shift 2 ;;
        --tftp-only) http=0; shift ;;
        --http) http=1; shift ;;
        --tftp-blocksize) blocksize="$2"; shift 2 ;;
        --tftp-dir) tftp="$2"; shift 2 ;;
        --image) image="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done
[ -n "$server" ] || { echo "--server is required" >&2; exit 1; }

case "$board" in
    # RAM starts at 0; U-Boot and the firmware live near the top of the first
    # GiB, the kernel is loaded at 0x80000.
    # The compressed image goes right after the 256 MiB disk image.
    rpi) img_addr=0x10000000; gz_addr=0x20000000; mkimage="$ROOT/build/u-boot/obj/tools/mkimage" ;;
    # 512 MB, less the GPU's share (gpu_mem=16): a 128 MiB image from
    # 128 MiB on, the compressed one after it. U-Boot sits near the top, the
    # Redox bootloader copies the RedoxFS partition into what is left.
    zero2w) img_addr=0x08000000; gz_addr=0x10000000; mkimage="$ROOT/build/u-boot/obj/tools/mkimage" ;;
    # RAM starts at 1 GiB.
    qemu) img_addr=0x50000000; gz_addr=0x60000000; mkimage="$ROOT/build/u-boot-qemu/obj/tools/mkimage" ;;
    *) echo "unknown board: $board" >&2; exit 1 ;;
esac
[ -x "$mkimage" ] || { echo "$mkimage missing, run scripts/build-uboot.sh ${board/zero2w/rpi}" >&2; exit 1; }
if [ -z "$image" ]; then
    if [ "$board" = zero2w ]; then image="$REDOX/build/aarch64/zero2w/harddrive.img"
    else image="$REDOX/build/aarch64/minimal/harddrive.img"; fi
fi
if [ -z "$http" ]; then
    if [ "$board" = zero2w ]; then http=0; else http=1; fi
fi
if [ -z "$blocksize" ] && [ "$board" = zero2w ]; then blocksize=512; fi
if [ "$http" = 1 ]; then http_url="http://$server:8080/"; else http_url=none; fi
if [ -z "$dir" ]; then
    if [ "$board" = zero2w ]; then dir=redox-zero2w; else dir=redox; fi
fi

bootloader="$(find "$REDOX/recipes/core/bootloader/target" -path '*stage/usr/lib/boot/bootloader-live.efi' | head -1)"
[ -n "$bootloader" ] || { echo "bootloader-live.efi missing, run scripts/build-image.sh" >&2; exit 1; }

# U-Boot unpacks the image from gz_addr to img_addr: it must fit in between.
if [ "$(wc -c < "$image")" -gt $((gz_addr - img_addr)) ]; then
    echo "$image is larger than $(((gz_addr - img_addr) >> 20)) MiB, the space U-Boot has for it" >&2
    exit 1
fi

mkdir -p "$tftp/$dir"
rm -f "$tftp/$dir/harddrive.img"
gzip -9 -c "$image" > "$tftp/$dir/harddrive.img.gz"
cp "$bootloader" "$tftp/$dir/bootloader-live.efi"

# The build manifest goes with the image it describes (docs/versioning.md).
manifest="$(grep -l "\"harddrive.img\" = \"$(sha256sum < "$image" | cut -d' ' -f1)\"" \
    "$ROOT"/build/manifests/*.toml 2>/dev/null | head -1 || true)"
if [ -n "$manifest" ]; then
    cp "$manifest" "$tftp/$dir/manifest.toml"
else
    rm -f "$tftp/$dir/manifest.toml"
    echo "warning: no manifest in build/manifests/ matches $image" >&2
fi

script="$tftp/$dir/netboot.cmd"
{
    echo "# Generated by scripts/netboot.sh"
    echo "if test -z \"\${redox_server}\"; then setenv redox_server $server; fi"
    echo "if test -z \"\${redox_dir}\"; then setenv redox_dir $dir/; fi"
    echo "if test -z \"\${redox_http}\"; then setenv redox_http $http_url; fi"
    echo "if test -z \"\${redox_img_addr}\"; then setenv redox_img_addr $img_addr; fi"
    echo "if test -z \"\${redox_gz_addr}\"; then setenv redox_gz_addr $gz_addr; fi"
    if [ -n "$blocksize" ]; then
        echo "if test -z \"\${tftpblocksize}\"; then setenv tftpblocksize $blocksize; fi"
    fi
    cat "$ROOT/config/netboot.cmd"
} > "$script"
"$mkimage" -q -A arm64 -T script -C none -n "redox-rpi netboot" -d "$script" "$tftp/$dir/netboot.scr"

# boot.scr for the SD card: fetch netboot.scr from the server and run it, so
# changes to the script only need the TFTP side.
cat > "$tftp/$dir/boot.cmd" <<EOF
# Generated by scripts/netboot.sh: network boot Redox from $server, $dir/
usb start
setenv autoload no
dhcp
setenv serverip $server
tftpboot \${scriptaddr} $dir/netboot.scr && source \${scriptaddr}
EOF
"$mkimage" -q -A arm64 -T script -C none -n "redox-rpi boot" -d "$tftp/$dir/boot.cmd" "$tftp/$dir/boot.scr"

echo "TFTP directory ready: $tftp/$dir"
ls -l "$tftp/$dir"
