#!/usr/bin/env bash
# mkmini.sh <variant dir> <livedisk yes|no> <extra unit>...
# A copy of the built initfs whose 90_initfs.target only needs the probe app
# (plus the extra units), repacked, in <variant dir> with kernel8.img and
# qemu.dtb from build/fw-direct.
set -euo pipefail
cd "$(dirname "$0")"
out=$1 live=$2; shift 2
B=../upstream/redox/recipes/core/base/target/aarch64-unknown-redox
rm -rf "$out"; mkdir -p "$out"
cp -a $B/build/initfs "$out/tree"
cp hello-app "$out/tree/bin/"
req='"00_runtime.target"'
for u in "$@"; do req="$req, \"$u\""; done
printf '[unit]\ndescription = "Boot-time probe app"\nrequires_weak = [%s]\n\n[service]\ncmd = "hello-app"\ntype = "oneshot_async"\n' "$req" > "$out/tree/lib/init.d/60_app.service"
req='"60_app.service"'
printf '[unit]\ndescription = "initfs finalized"\nrequires_weak = [%s]\n' "$req" > "$out/tree/lib/init.d/90_initfs.target"
$B/build/target/debug/redox-initfs-ar "$out/tree" $B/build/aarch64-unknown-redox/release/bootstrap -o "$out/initfs" >/dev/null
cp ../build/fw-direct/kernel8.img ../build/fw-direct/qemu.dtb "$out/"
cp "$out/initfs" "$out/initramfs"
if [ "$live" = yes ]; then
    img=../build/fw-direct/harddrive.img
    read -r start sectors < <(partx -g -o START,SECTORS -n 3 "$img")
    truncate -s %4096 "$out/initramfs"
    dd if="$img" bs=512 skip="$start" count="$sectors" status=none >> "$out/initramfs"
fi
echo "$out: initfs $(wc -c < "$out/initfs") bytes, initramfs $(wc -c < "$out/initramfs") bytes"
