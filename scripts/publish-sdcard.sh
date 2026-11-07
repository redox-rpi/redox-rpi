#!/usr/bin/env bash
# Publish a finished SD card image on the image server, the one way into
# /srv/tftp/sdcard (http://homenodellm:8080/sdcard/), so SwiftFlash
# (github.com/zehrer/SwiftFlashCLI, docs/build-server.md) can flash it with
#   swiftflash source add redox http://homenodellm:8080/sdcard/
#   sudo swiftflash redox:<family>
# Rules in docs/topics/ci.md, "Image server".
#
# Usage:
#   scripts/publish-sdcard.sh <card image> --family <family> [--version <v>]
#                             [--manifest <toml>] [--owner <topic>]
#                             [--commit <hash>] [--latest] [--no-uuid]
#
#   <card image>  a whole card, .img or .img.xz: from scripts/make-sdcard.sh,
#                 or made any other way (firmware-only cards)
#   --family      stable name of the image variant, e.g. redox-rpi-openfw-3bp;
#                 lower case, digits and "-", no version in it
#   --version     v<major>.<minor>… (git describe, a build ID) or YYYYMMDD…;
#                 default: the build ID of --manifest, else today's date,
#                 with b, c, … appended when that name is taken
#   --manifest    the build manifest of the image (build/manifests/*.toml);
#                 published next to it, and its build ID, commit and time
#                 go into .uuid and index.json
#   --owner       topic page that owns the image (shown in index.json)
#   --commit      commit for .uuid when there is no manifest
#   --latest      also point <family>-latest.img.xz (and .sha256,
#                 .manifest.toml) at it, for people; index.json leaves the
#                 links out
#   --no-uuid     the card has no FAT first partition; publish without
#                 /.uuid
#
# Steps: refuse a file name that is already published (never overwrite),
# write /.uuid (schemaVersion 1, only the image block: id = family, version
# = the version in the file name) into the FAT first partition of a copy,
# compress with xz -9, write .sha256, .manifest.toml and .meta.json, move the
# image into place last, and rewrite index.json (scripts/sdcard-index.py).
# SDCARD_DIR in the environment publishes elsewhere (for tests).
#
# Needs: xz, mtools, partx (util-linux), python3 3.11+.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIR="${SDCARD_DIR:-/srv/tftp/sdcard}"

card=""
family=""
version=""
manifest=""
owner=""
commit=""
latest=0
uuid=1
visit=""
while [ $# -gt 0 ]; do
    case "$1" in
        --family) family="$2"; shift 2 ;;
        --version) version="$2"; shift 2 ;;
        --manifest) manifest="$2"; shift 2 ;;
        --owner) owner="$2"; shift 2 ;;
        --commit) commit="$2"; shift 2 ;;
        --visit) visit="$2"; shift 2 ;;   # set by build-visit-images.sh
        --latest) latest=1; shift ;;
        --no-uuid) uuid=0; shift ;;
        -*) echo "unknown argument: $1" >&2; exit 1 ;;
        *) [ -z "$card" ] || { echo "only one card image" >&2; exit 1; }; card="$1"; shift ;;
    esac
done
die() { echo "publish-sdcard.sh: $*" >&2; exit 1; }
[ -n "$card" ] && [ -f "$card" ] || die "no card image given or $card missing"
[ -n "$family" ] || die "--family is required"
[[ "$family" =~ ^[a-z0-9][a-z0-9-]*[a-z0-9]$ ]] || die "family $family: only a-z, 0-9 and -"
[[ "$family" =~ -(v[0-9]+\.[0-9]+|[0-9]{8}) ]] && die "family $family contains a version"
[[ "$family" = *-latest ]] && die "family $family ends in -latest"
[ -z "$manifest" ] || [ -f "$manifest" ] || die "$manifest missing"

field() { sed -n "s/^$1 = \"\(.*\)\"$/\1/p" "$manifest" 2>/dev/null | head -1; }
build_id="" built="" experiments=""
if [ -n "$manifest" ]; then
    build_id="$(field build_id)"
    built="$(field built)"
    [ -n "$commit" ] || commit="$(field commit)"
    experiments="$(sed -n 's/^experiments = \[\(.*\)\]$/\1/p' "$manifest" | tr -d ' "' | sed 's/,$//')"
fi
[ -n "$built" ] || built="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

mkdir -p "$DIR"
exec 8> "$DIR/.publish.lock"
flock 8

# The name: never one that is published already.
taken() { [ -e "$DIR/$family-$1.img.xz" ] || [ -e "$DIR/$family-$1.img.xz.sha256" ]; }
if [ -n "$version" ]; then
    taken "$version" && die "$family-$version.img.xz is published already; a new build needs a new version"
elif [ -n "$build_id" ]; then
    version="$build_id"
    taken "$version" && die "$family-$version.img.xz is published already (same build ID); nothing replaced"
else
    day="$(date -u +%Y%m%d)"
    version="$day"
    for suffix in b c d e f g h i j k l m n o p q r s t u v w x y z; do
        taken "$version" || break
        version="$day$suffix"
    done
    taken "$version" && die "no free name for $family on $day"
fi
[[ "$version" =~ ^(v[0-9]+\.[0-9]+|[0-9]{8}) ]] || die "version $version must start with v<major>.<minor> or YYYYMMDD"
[[ "$version" =~ ^[A-Za-z0-9.+_-]+$ ]] || die "version $version: only letters, digits and . + _ -"
name="$family-$version.img.xz"

work="$(mktemp -d "$DIR/.publish-XXXXXX")"
trap 'rm -rf "$work"' EXIT
img="$work/card.img"
case "$card" in
    *.xz) xz -dc "$card" > "$img" ;;
    *) cp "$card" "$img" ;;
esac

if [ "$uuid" = 1 ]; then
    start="$(partx -g -o START -n 1 "$img" 2>/dev/null | tr -d ' ')" ||
        die "$card has no partition table; --no-uuid to publish without /.uuid"
    fat="$img@@$((start * 512))"
    minfo -i "$fat" :: > /dev/null 2>&1 ||
        die "partition 1 of $card is not FAT; --no-uuid to publish without /.uuid"
    python3 - "$family" "$version" "$built" "${commit:0:12}" > "$work/uuid" <<'EOF'
import json, sys
family, version, built, commit = sys.argv[1:]
image = {"id": family, "name": "Redox OS for Raspberry Pi (redox-rpi)",
         "version": version, "build": built}
if commit:
    image["commit"] = commit
print(json.dumps({"schemaVersion": 1, "image": image}, indent=2))
EOF
    mcopy -o -i "$fat" "$work/uuid" ::.uuid
fi

xz -T0 -9 -c "$img" > "$work/$name"
(cd "$work" && sha256sum "$name" > "$name.sha256")
[ -z "$manifest" ] || cp "$manifest" "$work/$name.manifest.toml"
python3 - "$family" "$version" "$built" "$commit" "$build_id" "$experiments" "$owner" "$visit" \
    > "$work/$name.meta.json" <<'EOF'
import json, sys
keys = ["family", "version", "date", "commit", "build_id", "experiments", "owner", "visit"]
meta = dict(zip(keys, sys.argv[1:]))
meta["commit"] = meta["commit"][:12]
meta["experiments"] = [e for e in meta["experiments"].split(",") if e]
print(json.dumps({k: v for k, v in meta.items() if v}, indent=2))
EOF

# Side files first, the image last: index.json and listings only count
# .img.xz files, so a reader never sees an image without its checksum.
for ext in sha256 manifest.toml meta.json; do
    [ -f "$work/$name.$ext" ] && mv "$work/$name.$ext" "$DIR/$name.$ext"
done
chmod 644 "$work/$name"
mv "$work/$name" "$DIR/$name"
if [ "$latest" = 1 ]; then
    for ext in "" .sha256 .manifest.toml; do
        [ -e "$DIR/$name$ext" ] && ln -sfn "$name$ext" "$DIR/$family-latest.img.xz$ext"
    done
fi

plan="$work/test-plan.md"
git -C "$ROOT" show origin/main:docs/test-plan.md > "$plan" 2>/dev/null ||
    cp "$ROOT/docs/test-plan.md" "$plan" 2>/dev/null || : > "$plan"
python3 "$ROOT/scripts/sdcard-index.py" "$DIR" "$plan"

echo "published: http://homenodellm:8080/sdcard/$name"
echo "sha256:    $(cut -d' ' -f1 "$DIR/$name.sha256")"
echo "flash:     sudo swiftflash redox:$family"
