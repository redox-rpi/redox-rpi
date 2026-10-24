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

cd "$REDOX"
# CI disables the cookbook's terminal UI, which needs a real terminal.
CI="${CI:-1}" make "${1:-all}" \
    PODMAN_BUILD=0 \
    ARCH=aarch64 \
    BOARD=raspi3bp \
    CONFIG_NAME=minimal \
    REPO_BINARY=1 \
    FILESYSTEM_CONFIG="$ROOT/config/raspi3bp.toml"
