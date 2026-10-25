#!/usr/bin/env bash
# Build U-Boot for the Raspberry Pi 3, 3B+ and Zero 2 W.
#
# Uses the upstream release below with rpi_arm64_defconfig, which takes the
# devicetree from the Pi firmware instead of embedding one, plus the options
# in config/u-boot.config. U-Boot is GPL-2.0; this script and the pinned tag
# are the corresponding source for any u-boot.bin we distribute.
#
# Needs: make, gcc-aarch64-linux-gnu, bison, flex, bc, swig, libssl-dev,
# libgnutls28-dev, python3-pyelftools, device-tree-compiler.
#
# Usage:
#   scripts/build-uboot.sh
#
# Result: build/u-boot/u-boot.bin and the devicetrees in build/u-boot/dtb/

set -euo pipefail

UBOOT_TAG="v2026.10"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/upstream/u-boot"
OUT="$ROOT/build/u-boot"
CROSS_COMPILE="${CROSS_COMPILE:-aarch64-linux-gnu-}"

if [ ! -d "$SRC" ]; then
    git clone --depth 1 --branch "$UBOOT_TAG" https://source.denx.de/u-boot/u-boot.git "$SRC"
fi
if [ "$(git -C "$SRC" describe --tags)" != "$UBOOT_TAG" ]; then
    git -C "$SRC" fetch --depth 1 origin tag "$UBOOT_TAG"
    git -C "$SRC" checkout -q "$UBOOT_TAG"
fi

mkdir -p "$OUT/obj" "$OUT/dtb"
make -C "$SRC" O="$OUT/obj" CROSS_COMPILE="$CROSS_COMPILE" rpi_arm64_defconfig
"$SRC/scripts/kconfig/merge_config.sh" -m -O "$OUT/obj" \
    "$OUT/obj/.config" "$ROOT/config/u-boot.config"
make -C "$SRC" O="$OUT/obj" CROSS_COMPILE="$CROSS_COMPILE" olddefconfig
make -C "$SRC" O="$OUT/obj" CROSS_COMPILE="$CROSS_COMPILE" -j"$(nproc)"

cp "$OUT/obj/u-boot.bin" "$OUT/u-boot.bin"
# Mainline devicetrees, used to boot this U-Boot in QEMU without Pi firmware.
find "$OUT/obj" -name 'bcm2837-rpi-*.dtb' -exec cp {} "$OUT/dtb/" \;
echo "u-boot: $OUT/u-boot.bin ($UBOOT_TAG)"
