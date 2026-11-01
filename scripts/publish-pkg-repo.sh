#!/usr/bin/env bash
# Publish redox-rpi's own packages as a pkg repository (issue #44).
#
# Takes packages from the cookbook's local repository of the last
# scripts/build-image.sh run, signs them again with the fixed redox-rpi
# repository key and writes them where the build host's HTTP server serves
# them:
#   <out>/id_ed25519.pub.toml                 public key, fetched by pkg
#   <out>/aarch64-unknown-redox/repo.toml     index: package = blake3
#   <out>/aarch64-unknown-redox/<name>.pkgar  and <name>.toml
#
# Images built with --experiment pkg-repo have /etc/pkg.d/40_redox-rpi, so
# pkg asks this repository before static.redox-os.org. Packages published
# earlier stay in the index; a new run replaces those it publishes again.
#
# The secret key is created once with
#   build/tools/bin/pkgar-keys gen --plaintext \
#       --skey ~/.config/redox-rpi/pkg-key/id_ed25519.toml \
#       --pkey ~/.config/redox-rpi/pkg-key/id_ed25519.pub.toml
# and never leaves the build host. The pkgar tool comes from
#   cargo install --locked --root build/tools pkgar --version 0.2.4 --features cli
#
# Usage:
#   scripts/publish-pkg-repo.sh [--out <dir>] [--key-dir <dir>] [<package>...]
#
#   default packages: our patched upstream packages and our own tools that
#   exist in the local repository; devshd is never published (unencrypted,
#   for development images only)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REDOX="$ROOT/upstream/redox"
TARGET=aarch64-unknown-redox
src="$REDOX/repo/$TARGET"
out=/srv/tftp/pkg
key_dir="$HOME/.config/redox-rpi/pkg-key"
pkgar="$ROOT/build/tools/bin/pkgar"
packages=()
while [ $# -gt 0 ]; do
    case "$1" in
        --out) out="$2"; shift 2 ;;
        --key-dir) key_dir="$2"; shift 2 ;;
        -*) echo "unknown argument: $1" >&2; exit 1 ;;
        *) packages+=("$1"); shift ;;
    esac
done
[ ${#packages[@]} -gt 0 ] || packages=(base base-initfs bootloader kernel relibc pkgutils
    redox-fatfs2 reboot timesync)
[ -x "$pkgar" ] || { echo "$pkgar missing, see the header of this script" >&2; exit 1; }
[ -f "$key_dir/id_ed25519.toml" ] || { echo "no key in $key_dir, see the header of this script" >&2; exit 1; }
build_pkey="$REDOX/build/id_ed25519.pub.toml"
[ -f "$build_pkey" ] || { echo "$build_pkey missing, run scripts/build-image.sh" >&2; exit 1; }
# Build ID of the newest image (its manifest), for repo.toml.
manifest="$(ls -t "$ROOT"/build/manifests/*-latest.toml 2>/dev/null | head -1 || true)"
release="$( [ -n "$manifest" ] && sed -n 's/^build_id = "\(.*\)"/\1/p' "$manifest" || true)"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$out/$TARGET"

# blake3 of a pkgar archive's entries, as in repo.toml: bytes 96..128 of the
# header (signature 64, public key 32, blake3 32).
header_blake3() {
    od -An -tx1 -j96 -N32 "$1" | tr -d ' \n'
}

for name in "${packages[@]}"; do
    [ "$name" = devshd ] && { echo "skipping devshd" >&2; continue; }
    [ -f "$src/$name.pkgar" ] || { echo "skipping $name: not in $src" >&2; continue; }
    rm -rf "$work/stage"
    mkdir -p "$work/stage"
    "$pkgar" extract --pkey "$build_pkey" --archive "$src/$name.pkgar" "$work/stage"
    "$pkgar" create --skey "$key_dir/id_ed25519.toml" --archive "$work/$name.pkgar" "$work/stage"
    hash="$(header_blake3 "$work/$name.pkgar")"
    size="$(stat -c %s "$work/$name.pkgar")"
    sed -e "s/^blake3 = .*/blake3 = \"$hash\"/" \
        -e "s/^network_size = .*/network_size = $size/" \
        "$src/$name.toml" > "$work/$name.toml"
    mv "$work/$name.pkgar" "$out/$TARGET/$name.pkgar"
    mv "$work/$name.toml" "$out/$TARGET/$name.toml"
    echo "$name $hash"
done

cp "$key_dir/id_ed25519.pub.toml" "$out/id_ed25519.pub.toml"

# The index lists every package in the output directory.
{
    echo "build_id = \"${release:-unknown}\""
    echo
    echo "[packages]"
    for toml in "$out/$TARGET"/*.toml; do
        name="$(basename "$toml" .toml)"
        [ "$name" = repo ] && continue
        echo "$name = \"$(sed -n 's/^blake3 = "\(.*\)"/\1/p' "$toml")\""
    done
} > "$work/repo.toml"
mv "$work/repo.toml" "$out/$TARGET/repo.toml"
echo "repository: $out ($(sed '1,/^\[packages\]/d' "$out/$TARGET/repo.toml" | grep -c .) packages)"
