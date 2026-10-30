#!/usr/bin/env bash
# Build the Redox aarch64 kernel with this repository's patches.
#
# Works on macOS and Linux. Needs git, rustup and make. Sources are cloned
# into upstream/ on first use, at the commits in config/upstream.lock;
# existing clones are used as they are, so local changes there end up in the
# build.
#
# Usage:
#   scripts/build-kernel.sh [--latest]
#
#   --latest   clone the newest upstream kernel instead of the pinned commit
#              (CI checks with it whether our patches still work upstream)
#
# Result: upstream/kernel/build/aarch64/kernel

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UPSTREAM="$ROOT/upstream"
KERNEL="$UPSTREAM/kernel"
FDT="$UPSTREAM/fdt-crate"
LOCK="$ROOT/config/upstream.lock"
pinned() { awk -v n="$1" '$1 == n { print $2 }' "$LOCK"; }
# The fdt revision the kernel pins in its Cargo.toml; our patches apply to it.
FDT_REV="$(pinned fdt)"

kernel_rev="$(pinned kernel)"
case "${1:-}" in
    --latest) kernel_rev=HEAD ;;
    "") ;;
    *) echo "unknown argument: $1" >&2; exit 1 ;;
esac

mkdir -p "$UPSTREAM"

if [ ! -d "$KERNEL" ]; then
    git init -q "$KERNEL"
    git -C "$KERNEL" fetch -q --depth 1 https://gitlab.redox-os.org/redox-os/kernel.git "$kernel_rev"
    git -C "$KERNEL" checkout -q FETCH_HEAD
    git -C "$KERNEL" apply "$ROOT"/patches/kernel/*.patch
fi
echo "kernel source: $(git -C "$KERNEL" rev-parse HEAD)"

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
