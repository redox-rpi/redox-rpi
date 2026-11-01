#!/usr/bin/env bash
# Build U-Boot for the Raspberry Pi 3, 3B+ and Zero 2 W, or for QEMU's virt
# machine to test the network boot flow without a Pi.
#
# Uses the upstream release below with rpi_arm64_defconfig, which takes the
# devicetree from the Pi firmware instead of embedding one, plus the options
# in config/u-boot.config, with the fixes in patches/u-boot/ applied on top.
# U-Boot is GPL-2.0; this script, the pinned tag and those patches are the
# corresponding source for any u-boot.bin we distribute.
#
# Needs: make, gcc-aarch64-linux-gnu, bison, flex, bc, swig, libssl-dev,
# libgnutls28-dev, python3-pyelftools, device-tree-compiler.
#
# Usage:
#   scripts/build-uboot.sh [rpi|qemu]    (default: rpi)
#   scripts/build-uboot.sh rpi-net <server-ip>
#
# Result: build/u-boot/u-boot.bin and the devicetrees in build/u-boot/dtb/,
# or build/u-boot-qemu/u-boot.bin for qemu.
#
# rpi-net builds build/u-boot-net/u-boot.bin for a Pi without SD card, which
# the Pi firmware loads over TFTP (scripts/netboot-firmware.sh). There is no
# boot.scr then, so its boot command fetches redox/netboot.scr from
# <server-ip> itself.

set -euo pipefail

UBOOT_TAG="v2026.10"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/upstream/u-boot"
case "${1:-rpi}" in
    rpi) DEFCONFIG=rpi_arm64_defconfig; OUT="$ROOT/build/u-boot" ;;
    rpi-net)
        DEFCONFIG=rpi_arm64_defconfig; OUT="$ROOT/build/u-boot-net"
        server="${2:?rpi-net needs the server address}" ;;
    qemu) DEFCONFIG=qemu_arm64_defconfig; OUT="$ROOT/build/u-boot-qemu" ;;
    *) echo "unknown target: $1" >&2; exit 1 ;;
esac
CROSS_COMPILE="${CROSS_COMPILE:-aarch64-linux-gnu-}"

if [ ! -d "$SRC" ]; then
    git clone --depth 1 --branch "$UBOOT_TAG" https://source.denx.de/u-boot/u-boot.git "$SRC"
fi
if ! git -C "$SRC" rev-parse -q --verify "refs/tags/$UBOOT_TAG" >/dev/null; then
    git -C "$SRC" fetch --depth 1 origin tag "$UBOOT_TAG"
fi
# The pinned tag plus our fixes, from a clean tree every time: a tree left
# with other or older changes to the same files still builds.
git -C "$SRC" checkout -q -f "$UBOOT_TAG"
git -C "$SRC" clean -qfd
for patch in "$ROOT"/patches/u-boot/*.patch; do
    [ -e "$patch" ] || continue
    git -C "$SRC" apply "$patch"
    echo "u-boot: applied $(basename "$patch")"
done

mkdir -p "$OUT/obj" "$OUT/dtb"
fragments=("$ROOT/config/u-boot.config")
if [ -n "${server:-}" ]; then
    # The same steps as boot.scr from scripts/netboot.sh.
    cat > "$OUT/net.config" <<EOF
CONFIG_USE_BOOTCOMMAND=y
CONFIG_BOOTCOMMAND="usb start; setenv autoload no; dhcp; setenv serverip $server; tftpboot \${scriptaddr} redox/netboot.scr && source \${scriptaddr}"
EOF
    fragments+=("$OUT/net.config")
fi
make -C "$SRC" O="$OUT/obj" CROSS_COMPILE="$CROSS_COMPILE" "$DEFCONFIG"
"$SRC/scripts/kconfig/merge_config.sh" -m -O "$OUT/obj" \
    "$OUT/obj/.config" "${fragments[@]}"
make -C "$SRC" O="$OUT/obj" CROSS_COMPILE="$CROSS_COMPILE" olddefconfig
make -C "$SRC" O="$OUT/obj" CROSS_COMPILE="$CROSS_COMPILE" -j"$(nproc)"

cp "$OUT/obj/u-boot.bin" "$OUT/u-boot.bin"
# Mainline devicetrees, used to boot this U-Boot in QEMU without Pi firmware.
find "$OUT/obj" -name 'bcm2837-rpi-*.dtb' -exec cp {} "$OUT/dtb/" \;
echo "u-boot: $OUT/u-boot.bin ($UBOOT_TAG)"
