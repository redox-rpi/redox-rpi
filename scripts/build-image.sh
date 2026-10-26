#!/usr/bin/env bash
# Build a Redox image for the Raspberry Pi 3 Model B+ with this repository's
# patches.
#
# Needs an x86_64 Linux host prepared as described in docs/build-host.md.
# Sources are cloned into upstream/ on first use.
#
# Usage:
#   scripts/build-image.sh [make target]   (default: all)
#
# Result: upstream/redox/build/aarch64/raspi3bp/harddrive.img

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UPSTREAM="$ROOT/upstream"
REDOX="$UPSTREAM/redox"
FDT="$UPSTREAM/fdt-crate"
FDT_REV="8fa4388e056e48826e8898557a1c8600f9ee5d6a"
KERNEL_RECIPE="$REDOX/recipes/core/kernel"

mkdir -p "$UPSTREAM"

# The cookbook does not notice changed patches in an already fetched recipe.
# Drop its source and build when our patch set for it changed.
refresh_on_patch_change() {
    local recipe="$1" patches="$2"
    local stamp="$recipe/.redox-rpi-patches"
    local sum
    sum="$(cat "$patches"/*.patch | sha256sum | cut -d' ' -f1)"
    if [ "$(cat "$stamp" 2>/dev/null)" != "$sum" ]; then
        rm -rf "$recipe/source" "$recipe/target"
        echo "$sum" > "$stamp"
    fi
}

if [ ! -d "$REDOX" ]; then
    git clone --depth 1 https://gitlab.redox-os.org/redox-os/redox.git "$REDOX"
fi

if [ ! -d "$FDT" ]; then
    git init -q "$FDT"
    git -C "$FDT" fetch -q --depth 1 https://github.com/repnop/fdt.git "$FDT_REV"
    git -C "$FDT" checkout -q FETCH_HEAD
    git -C "$FDT" apply "$ROOT"/patches/fdt/*.patch
fi

# Kernel recipe: apply our kernel patches and build against the patched fdt.
# Rewritten on every run so changes to the patches are picked up.
git -C "$REDOX" checkout -q -- recipes/core/kernel
refresh_on_patch_change "$KERNEL_RECIPE" "$ROOT/patches/kernel"
patches=""
for patch in "$ROOT"/patches/kernel/*.patch; do
    cp "$patch" "$KERNEL_RECIPE/"
    patches="$patches  \"$(basename "$patch")\",
"
done
cat > "$KERNEL_RECIPE/recipe.toml" <<EOF
[source]
git = "https://gitlab.redox-os.org/redox-os/kernel.git"
patches = [
$patches]

[build]
template = "custom"
script = """
cd "\${COOKBOOK_SOURCE}"
mkdir -p .cargo
cat > .cargo/config.toml <<END
[patch."https://github.com/repnop/fdt.git"]
fdt = { path = "$FDT" }
END
make install BUILD="\${COOKBOOK_BUILD}" DESTDIR="\${COOKBOOK_STAGE}"
"""
EOF

# Other recipes: add our patches to their [source] section.
for name in bootloader base; do
    recipe="$REDOX/recipes/core/$name"
    git -C "$REDOX" checkout -q -- "recipes/core/$name"
    refresh_on_patch_change "$recipe" "$ROOT/patches/$name"
    list=""
    for patch in "$ROOT/patches/$name"/*.patch; do
        cp "$patch" "$recipe/"
        list="$list\"$(basename "$patch")\", "
    done
    sed -i "/^git = /a patches = [ $list]" "$recipe/recipe.toml"
done

# Our own packages, built from this repository.
for tool in devshd reboot; do
    mkdir -p "$REDOX/recipes/other/$tool"
    cat > "$REDOX/recipes/other/$tool/recipe.toml" <<EOF
[source]
path = "$ROOT/tools/$tool"

[build]
template = "cargo"
EOF
done

cd "$REDOX"
# make only rebuilds packages when repo.tag is older than what it tracks,
# which does not include our patches. The cookbook itself skips unchanged
# recipes, so letting it check every time is cheap.
rm -f build/aarch64/minimal/repo.tag
# CI disables the cookbook's terminal UI, which needs a real terminal.
CI="${CI:-1}" make "${1:-all}" \
    PODMAN_BUILD=0 \
    ARCH=aarch64 \
    BOARD=raspi3bp \
    CONFIG_NAME=minimal \
    REPO_BINARY=1 \
    FILESYSTEM_CONFIG="$ROOT/config/raspi3bp.toml"
