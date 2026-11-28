#!/usr/bin/env bash
# The console image: a small direct-boot image (kernel8.img and initramfs
# for the Pi firmware, no U-Boot) for console work and boot-time tests. The
# Pi firmware reads the initramfs from the SD card at about 1 MB/s
# (rpi-open-firmware#34), so a 48 MiB initramfs boots much sooner than the
# 260 MiB one of a full image (row 467: `redox login:` about 68 s after power
# on, against about 4 minutes).
#
# Made from a network-boot image built by scripts/build-image.sh, in the
# same clone (it needs base's initfs build folder):
#   1. the initfs again, from the same files with every program stripped of
#      its symbols (llvm-strip --strip-unneeded, about 27 -> 18 MB);
#   2. the root filesystem without what config/console-prune.txt lists
#      (documentation, headers, static libraries, TLS/HTTP, drivers for other
#      hardware, ...), every remaining ELF file stripped, copied into a new
#      RedoxFS of --size MiB (default 32);
#   (redoxfs-clone takes the source first, although its usage text says
#   otherwise: the other order formats the source.)
#   3. kernel8.img from scripts/fw-direct-boot.sh; the banner says
#      "direct console".
#
# --minimal also removes what config/console-prune-minimal.txt lists
# (network, sound, desktop, developer tools): the minimal Redox of the
# firmware release card (rpi-open-firmware#56). Its default source image is
# the release-netboot build (scripts/build-image.sh --release-netboot), which
# has no devshd. It also leaves out of the initfs what
# config/console-prune-initfs.txt lists (drivers for hardware the Pi does
# not have, sound), and takes their services out of the initfs targets.
#
# --root-on-card (#615 option A): the initramfs is the initfs alone, and the
# RedoxFS (console.fs) goes into a partition of the card. The firmware then
# reads about 13 MiB instead of about 49. <out>/cmdline-root.txt has the
# words for cmdline.txt that name the partition (REDOXFS_UUID,
# REDOXFS_BLOCK); 50_rootfs mounts it through bcm2835-sdhostd, as on the
# U-Boot cards. Changes on the card are kept.
#
# Writes <out>/kernel8.img, <out>/initramfs (initfs, then the RedoxFS),
# <out>/console.fs and <out>/manifest.txt (source build, prune lists, the
# boot files' SHA-256 and every file of the root filesystem with its size). Copy the two boot files to a direct card's FAT partition
# (make-sdcard.sh --direct) or give them to the firmware. --test boots them
# in QEMU raspi3b and waits for `redox login:`.
#
# Needs what scripts/fw-direct-boot.sh needs, plus FUSE (/dev/fuse,
# fusermount) for the redoxfs mount, as the upstream build's `make mount`.
#
# Usage:
#   scripts/console-image.sh [--minimal] [--root-on-card] [--image <harddrive.img>]
#                            [--out <dir>] [--size <MiB>] [--test]

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REDOX="$ROOT/upstream/redox"
image=""
out="$ROOT/build/console"
size=32
test=no
minimal=no
root_on_card=no
while [ $# -gt 0 ]; do
    case "$1" in
        --image) image="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        --size) size="$2"; shift 2 ;;
        --test) test=yes; shift ;;
        --minimal) minimal=yes; shift ;;
        --root-on-card) root_on_card=yes; shift ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done
prune=("$ROOT/config/console-prune.txt")
if [ "$minimal" = yes ]; then
    prune+=("$ROOT/config/console-prune-minimal.txt")
    [ -n "$image" ] || image="$REDOX/build/aarch64/release-netboot/harddrive.img"
fi
[ -n "$image" ] || image="$REDOX/build/aarch64/minimal/harddrive.img"
hint=""; [ "$minimal" = no ] || hint=" --release-netboot"
[ -f "$image" ] || { echo "$image missing, run scripts/build-image.sh$hint" >&2; exit 1; }
[ "$minimal" = no ] || [ "$image" != "$REDOX/build/aarch64/minimal/harddrive.img" ] ||
    echo "warning: --minimal from the network-boot test build, which has devshd; use --release-netboot" >&2

fstools="$REDOX/build/fstools/bin"
base="$REDOX/recipes/core/base/target/aarch64-unknown-redox"
strip="$REDOX/prefix/aarch64-unknown-redox/clang-install/bin/llvm-strip"
initfs_ar="$base/build/target/debug/redox-initfs-ar"
tool="$ROOT/tools/redoxfs-file/target/release/redoxfs-file"
for need in "$fstools/redoxfs" "$fstools/redoxfs-clone" "$strip" "$initfs_ar" \
            "$base/build/initfs" "$base/build/aarch64-unknown-redox/release/bootstrap"; do
    [ -e "$need" ] || { echo "$need missing: build the image in this clone first" >&2; exit 1; }
done

mkdir -p "$out"
out="$(cd "$out" && pwd)"
work="$(mktemp -d "$out/work.XXXXXX")"
mnt="$work/mnt"
cleanup() {
    mountpoint -q "$mnt" 2>/dev/null && fusermount -u "$mnt"
    rm -rf "$work"
}
trap cleanup EXIT

# kernel8.img, and a copy of the image whose banner says "direct".
"$ROOT/scripts/fw-direct-boot.sh" --image "$image" --out "$work/fw" > /dev/null
cp "$work/fw/kernel8.img" "$out/kernel8.img"

# 1. The initfs, stripped (the bootstrap stays as it is). Its files come
# from base's build folder; check
# that they are the ones in the image.
"$tool" "$image" get usr/lib/boot/initfs "$work/initfs.image" > /dev/null
if ! cmp -s "$work/initfs.image" "$base/build/initfs.img"; then
    echo "the initfs in $image is not base's last build: build the image again in this clone" >&2
    exit 1
fi
cp -a "$base/build/initfs" "$work/initfs"
bootstrap="$base/build/aarch64-unknown-redox/release/bootstrap"
# The unchanged files must give the image's initfs byte for byte, so the
# stripped one differs only in the symbols.
"$initfs_ar" "$work/initfs" "$bootstrap" -o "$work/initfs.rebuilt" > /dev/null
cmp -s "$work/initfs.rebuilt" "$work/initfs.image" || {
    echo "redox-initfs-ar does not rebuild the image's initfs from base's build folder" >&2
    exit 1
}
# --minimal: no drivers for hardware the Pi does not have. A removed service
# also leaves the targets that list it, so init does not look for it.
initfs_prune="$ROOT/config/console-prune-initfs.txt"
if [ "$minimal" = yes ]; then
    while read -r pattern; do
        pattern="${pattern%%#*}"
        pattern="${pattern%"${pattern##*[![:space:]]}"}"
        [ -n "$pattern" ] || continue
        # shellcheck disable=SC2086
        for path in $(cd "$work/initfs" && echo $pattern); do
            [ -e "$work/initfs/$path" ] || continue
            rm -rf -- "${work:?}/initfs/$path"
            case "$path" in
                lib/init.d/*.service)
                    sed -i "/\"$(basename "$path")\"/d" "$work/initfs"/lib/init.d/*.target ;;
            esac
        done
    done < "$initfs_prune"
fi
find "$work/initfs" -type f -exec sh -c 'head -c 4 "$1" | grep -q "ELF"' sh {} \; \
    -exec "$strip" --strip-unneeded {} \;
"$initfs_ar" "$work/initfs" "$bootstrap" -o "$work/initfs.small" > /dev/null

# 2. The root filesystem, pruned and stripped.
read -r start sectors < <(partx -g -o START,SECTORS -n 3 "$work/fw/harddrive.img")
dd if="$work/fw/harddrive.img" of="$work/root.fs" bs=512 skip="$start" count="$sectors" status=none
mkdir "$mnt"
"$fstools/redoxfs" "$work/root.fs" "$mnt" > "$work/redoxfs.log" 2>&1 &
for _ in $(seq 1 50); do mountpoint -q "$mnt" && break; sleep 0.1; done
mountpoint -q "$mnt" || { cat "$work/redoxfs.log" >&2; echo "cannot mount the root filesystem" >&2; exit 1; }
(
    cd "$mnt"
    shopt -s nullglob
    while read -r pattern; do
        pattern="${pattern%%#*}"
        pattern="${pattern%"${pattern##*[![:space:]]}"}"
        [ -n "$pattern" ] || continue
        # Unquoted on purpose: the pattern may be a glob.
        # shellcheck disable=SC2086
        rm -rf -- $pattern
    done < <(cat "${prune[@]}")
    find . -type f -size +4k -exec sh -c 'head -c 4 "$1" | grep -q "ELF"' sh {} \; \
        -exec "$strip" --strip-unneeded {} \;
    for file in etc/issue etc/redox-rpi-release etc/redox-rpi-manifest; do
        [ -f "$file" ] || continue
        # Rewritten in place: sed -i cannot keep the owner on the FUSE mount.
        sed -E -e 's/^(redox-rpi .*) \(direct, /\1 (direct console, /' \
            -e 's/^IMAGE="direct"$/IMAGE="direct-console"/' \
            -e 's/^image = "direct"$/image = "direct-console"/' "$file" > "$work/banner"
        cat "$work/banner" > "$file"
    done
)
used_kib="$(du -sk "$mnt" | cut -f1)"
release="$(cat "$mnt/etc/redox-rpi-release" 2>/dev/null || true)"
(cd "$mnt" && find . -type f -printf '%P %s\n' | LC_ALL=C sort) > "$work/files.txt"
fusermount -u "$mnt"
wait

truncate -s 0 "$out/console.fs"
truncate -s "${size}M" "$out/console.fs"
if ! "$fstools/redoxfs-clone" "$work/root.fs" "$out/console.fs" > "$work/clone.log" 2>&1; then
    tail -5 "$work/clone.log" >&2
    echo "the files ($((used_kib / 1024)) MiB) do not fit in --size $size MiB" >&2
    exit 1
fi

# 3. The initramfs: the initfs, then the RedoxFS at the next 4 KiB boundary
# (where rpi-boot-stub looks for it), as scripts/fw-direct-boot.sh does.
# With --root-on-card the initfs alone, and the words that name the
# partition for cmdline.txt (the UUID as rpi-boot-stub writes it, from the
# RedoxFS header).
cp "$work/initfs.small" "$out/initramfs"
rm -f "$out/cmdline-root.txt"
if [ "$root_on_card" = yes ]; then
    uuid="$(od -A n -t x1 -j 16 -N 16 "$out/console.fs" | tr -d ' \n' |
        sed -E 's/^(.{8})(.{4})(.{4})(.{4})(.{12})$/\1-\2-\3-\4-\5/')"
    [[ "$uuid" =~ ^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$ ]] || { echo "no RedoxFS UUID in console.fs" >&2; exit 1; }
    echo "REDOXFS_UUID=$uuid REDOXFS_BLOCK=0000000000000000" > "$out/cmdline-root.txt"
else
    truncate -s %4096 "$out/initramfs"
    cat "$out/console.fs" >> "$out/initramfs"
fi
sha() { sha256sum "$1" | cut -d' ' -f1; }
{
    echo "# Console image (scripts/console-image.sh$([ "$minimal" = yes ] && echo " --minimal")), $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "source_image $(basename "$image") $(sha "$image")"
    echo "$release" | sed 's/^/release /'
    for f in "${prune[@]}"; do echo "prune ${f#"$ROOT"/} $(sha "$f")"; done
    [ "$minimal" = no ] || echo "prune_initfs ${initfs_prune#"$ROOT"/} $(sha "$initfs_prune")"
    [ "$root_on_card" = no ] || echo "root_on_card console.fs $(sha "$out/console.fs") $(cat "$out/cmdline-root.txt")"
    echo "kernel8.img $(wc -c < "$out/kernel8.img") $(sha "$out/kernel8.img")"
    echo "initramfs $(wc -c < "$out/initramfs") $(sha "$out/initramfs")"
    echo "initfs $(wc -c < "$work/initfs.small")"
    echo "root_used_kib $used_kib in ${size} MiB"
    echo "# root filesystem: path size"
    cat "$work/files.txt"
} > "$out/manifest.txt"
[ "$root_on_card" = no ] || echo "root on the card: console.fs as a partition, cmdline.txt: $(cat "$out/cmdline-root.txt")"
echo "console image: initfs $(($(wc -c < "$work/initfs.small") >> 10)) KiB, root $((used_kib >> 10)) MiB in ${size} MiB," \
     "initramfs $(($(wc -c < "$out/initramfs") >> 20)) MiB, kernel8.img $(wc -c < "$out/kernel8.img") bytes, in $out"

[ "$test" = yes ] || exit 0
[ "$root_on_card" = no ] || { echo "--test boots the live disk only, not --root-on-card" >&2; exit 1; }
# With the framebuffer size a Pi's cmdline.txt gives (row 467).
"$ROOT/scripts/fw-direct-boot.sh" --out "$out" --boot-only --test --append RPI_FB=1280x720
