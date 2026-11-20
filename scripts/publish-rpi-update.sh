#!/usr/bin/env bash
# Publish a build for tools/rpi-update: the boot files of a direct-boot SD
# card (make-sdcard.sh --direct, issue #15) in a directory the web server on
# the build host serves, so a running Pi can fetch them without a new card.
#
# Writes <out>/kernel8.img, <out>/initramfs (scripts/fw-direct-boot.sh),
# <out>/SHA256SUMS and <out>/BUILD_ID (from /etc/redox-rpi-release in the
# image). The files are replaced one by one, SHA256SUMS last.
#
# Usage:
#   scripts/publish-rpi-update.sh [--image <harddrive.img>] [--out <dir>]
#
#   --image  default: the network-boot image of scripts/build-image.sh
#   --out    default: /srv/tftp/update (http://<host>:8080/update)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
image="$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img"
out=/srv/tftp/update

while [ $# -gt 0 ]; do
    case "$1" in
        --image) image="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        *) sed -n '2,/^set /p' "$0" | sed '$d; s/^# \{0,1\}//'; exit 2 ;;
    esac
done
[ -f "$image" ] || { echo "$image missing, run scripts/build-image.sh" >&2; exit 1; }

work="$ROOT/build/rpi-update"
"$ROOT/scripts/fw-direct-boot.sh" --image "$image" --out "$work"

tool="$ROOT/tools/redoxfs-file/target/release/redoxfs-file"
[ -x "$tool" ] || cargo build --release --manifest-path "$ROOT/tools/redoxfs-file/Cargo.toml"
"$tool" "$image" get /etc/redox-rpi-release "$work/redox-rpi-release"
build_id="$(sed -n 's/^BUILD_ID="\{0,1\}\([^"]*\)"\{0,1\}$/\1/p' "$work/redox-rpi-release")"
[ -n "$build_id" ] || { echo "no BUILD_ID in /etc/redox-rpi-release of $image" >&2; exit 1; }
echo "$build_id" > "$work/BUILD_ID"
(cd "$work" && sha256sum kernel8.img initramfs > SHA256SUMS)

mkdir -p "$out"
for file in kernel8.img initramfs BUILD_ID SHA256SUMS; do
    cp "$work/$file" "$out/.$file.new"
    mv "$out/.$file.new" "$out/$file"
done
echo "published $build_id to $out"
cat "$out/SHA256SUMS"
