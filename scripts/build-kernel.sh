#!/usr/bin/env bash
# Build the Redox aarch64 kernel with this repository's patches.
#
# Works on macOS and Linux. Needs git, rustup and make. Sources are cloned
# into upstream/ on first use; existing clones are used as they are, so local
# changes there end up in the build.
#
# Usage:
#   scripts/build-kernel.sh
#
# Result: upstream/kernel/build/aarch64/kernel

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UPSTREAM="$ROOT/upstream"
KERNEL="$UPSTREAM/kernel"
FDT="$UPSTREAM/fdt-crate"
# The fdt revision the kernel pins in its Cargo.toml; our patches apply to it.
FDT_REV="8fa4388e056e48826e8898557a1c8600f9ee5d6a"

mkdir -p "$UPSTREAM"

if [ ! -d "$KERNEL" ]; then
    git clone --depth 1 https://gitlab.redox-os.org/redox-os/kernel.git "$KERNEL"
    git -C "$KERNEL" apply "$ROOT"/patches/kernel/*.patch
fi

if [ ! -d "$FDT" ]; then
    git init -q "$FDT"
    git -C "$FDT" fetch -q --depth 1 https://github.com/repnop/fdt.git "$FDT_REV"
    git -C "$FDT" checkout -q FETCH_HEAD
    git -C "$FDT" apply "$ROOT"/patches/fdt/*.patch
fi

# Point the kernel's fdt dependency at the patched clone.
mkdir -p "$KERNEL/.cargo"
cat > "$KERNEL/.cargo/config.toml" <<EOF
[patch."https://github.com/repnop/fdt.git"]
fdt = { path = "$FDT" }
EOF

# Installs the toolchain pinned in rust-toolchain.toml if needed.
toolchain="$(cd "$KERNEL" && rustup show active-toolchain | cut -d' ' -f1)"
rustup component add --toolchain "$toolchain" rust-src llvm-tools >/dev/null
sysroot="$(cd "$KERNEL" && rustc --print sysroot)"
objcopy="$(find "$sysroot/lib/rustlib" -name llvm-objcopy -type f | head -1)"

make -C "$KERNEL" ARCH=aarch64 OBJCOPY="$objcopy" "$KERNEL/build/aarch64/kernel"
echo "kernel: $KERNEL/build/aarch64/kernel"
