#!/usr/bin/env bash
# Build a Redox image for the Raspberry Pi 3 Model B+ with this repository's
# patches.
#
# Needs an x86_64 Linux host prepared as described in docs/build-host.md.
# Sources are cloned into upstream/ on first use and pinned to the commits in
# config/upstream.lock. See docs/versioning.md for how a build is identified.
#
# Usage:
#   scripts/build-image.sh [--sdcard | --zero2w] [--experiment <name>]...
#                          [--release <name>] [--allow-dirty]
#                          [make target]   (default: all)
#
#   --sdcard      the image that boots from the SD card
#                 (config/raspi3bp-sdcard.toml) instead of the network-boot
#                 image (config/raspi3bp.toml)
#   --zero2w      the smaller network-boot image for the Pi Zero 2 W
#                 (config/raspizero2w.toml)
#   --experiment  also apply patches/experiments/<name>/ (repeatable): its
#                 patches, and its config.toml (packages, files) if it has one
#   --release     name of a release or release candidate (v0.3.0-rc2): shown
#                 above the login prompt, as RELEASE in /etc/redox-rpi-release
#                 and as release in the manifest (docs/release-process.md)
#   --allow-dirty build even with uncommitted changes to patches/, config/,
#                 scripts/ or tools/, or with recipe sources that differ from
#                 their pinned commit plus patches; the differences are saved
#                 next to the manifest and the build ID gets "-dirty"
#
# Result: upstream/redox/build/aarch64/minimal/harddrive.img (network boot) or
#         upstream/redox/build/aarch64/sdcard/harddrive.img (--sdcard),
#         upstream/redox/build/aarch64/zero2w/harddrive.img (--zero2w),
#         and its manifest in build/manifests/

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UPSTREAM="$ROOT/upstream"
REDOX="$UPSTREAM/redox"
FDT="$UPSTREAM/fdt-crate"
KERNEL_RECIPE="$REDOX/recipes/core/kernel"
LOCK="$ROOT/config/upstream.lock"
MANIFESTS="$ROOT/build/manifests"

CONFIG_NAME=minimal
CONFIG="$ROOT/config/raspi3bp.toml"
experiments=()
allow_dirty=0
release=""
while [ $# -gt 0 ]; do
    case "$1" in
        --sdcard)
            CONFIG_NAME=sdcard
            CONFIG="$ROOT/config/raspi3bp-sdcard.toml"
            shift ;;
        --zero2w)
            CONFIG_NAME=zero2w
            CONFIG="$ROOT/config/raspizero2w.toml"
            shift ;;
        --experiment)
            [ -d "$ROOT/patches/experiments/$2" ] || { echo "no patches/experiments/$2" >&2; exit 1; }
            experiments+=("$2")
            shift 2 ;;
        --release)
            [[ "${2:-}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-rc[0-9]+)?$ ]] || { echo "--release needs a name like v0.3.0 or v0.3.0-rc2" >&2; exit 1; }
            release="$2"
            shift 2 ;;
        --allow-dirty) allow_dirty=1; shift ;;
        -*) echo "unknown argument: $1" >&2; exit 1 ;;
        *) break ;;
    esac
done
image_name=netboot
[ "$CONFIG_NAME" = minimal ] || image_name="$CONFIG_NAME"

# One build at a time: builds share upstream/ and build/ (docs/test-plan.md).
# Under scripts/with-build-lock.sh the caller already holds the lock.
if [ -z "${REDOX_RPI_BUILD_LOCK:-}" ]; then
    mkdir -p "$ROOT/build"
    exec 9> "$ROOT/build/.build-image.lock"
    if ! flock -n 9; then
        echo "another build is running in $ROOT, waiting for it" >&2
        flock 9
    fi
    export REDOX_RPI_BUILD_LOCK=1
fi

# Commit of an upstream source from config/upstream.lock.
pin() {
    local rev
    rev="$(awk -v name="$1" '$1 == name { print $2 }' "$LOCK")"
    [ -n "$rev" ] || { echo "$1 missing in $LOCK" >&2; exit 1; }
    echo "$rev"
}

# Patches for a component, in the order they are applied: ours, then those of
# each selected experiment.
patch_files() {
    local name="$1" exp patch
    for patch in "$ROOT/patches/$name"/*.patch; do
        [ -e "$patch" ] && echo "$patch"
    done
    for exp in "${experiments[@]}"; do
        for patch in "$ROOT/patches/experiments/$exp/$name"/*.patch; do
            [ -e "$patch" ] && echo "$patch"
        done
    done
    return 0
}

# File name a patch gets in the recipe directory; experiment patches are
# prefixed so they cannot collide with ours.
recipe_patch_name() {
    case "$1" in
        "$ROOT/patches/experiments/"*)
            local rel="${1#"$ROOT/patches/experiments/"}"
            echo "exp-${rel%%/*}-$(basename "$1")" ;;
        *) basename "$1" ;;
    esac
}

dirty=0
drift_report="$(mktemp)"
trap 'rm -f "$drift_report"' EXIT

# Compare a source checkout with its commit plus patches. Lock files and the
# .cargo directory are left out: cargo and the kernel recipe write them.
check_drift() {
    local label="$1" src="$2"
    shift 2
    [ -d "$src/.git" ] || return 0
    local diff
    diff="$(
        export GIT_INDEX_FILE
        GIT_INDEX_FILE="$(mktemp)"
        trap 'rm -f "$GIT_INDEX_FILE"' EXIT
        git -C "$src" read-tree HEAD
        if [ $# -gt 0 ] && ! git -C "$src" apply --cached "$@" 2>/dev/null; then
            echo "patches do not apply to the checkout"
            exit 0
        fi
        expected="$(git -C "$src" write-tree)"
        git -C "$src" read-tree HEAD
        git -C "$src" add -A .
        actual="$(git -C "$src" write-tree)"
        git -C "$src" diff "$expected" "$actual" -- . ':!Cargo.lock' ':!.cargo'
    )"
    if [ -n "$diff" ]; then
        echo "$label: source differs from its commit plus patches" >&2
        { echo "### $label ($src)"; echo "$diff"; } >> "$drift_report"
        dirty=1
    fi
}

mkdir -p "$UPSTREAM"

# The build system and recipe collection, at the pinned commit.
redox_rev="$(pin redox)"
if [ ! -d "$REDOX" ]; then
    git clone -q --depth 1 https://gitlab.redox-os.org/redox-os/redox.git "$REDOX"
fi
if [ "$(git -C "$REDOX" rev-parse HEAD)" != "$redox_rev" ]; then
    git -C "$REDOX" checkout -q -- recipes/core
    git -C "$REDOX" fetch -q --depth 1 origin "$redox_rev"
    git -C "$REDOX" checkout -q "$redox_rev"
fi

fdt_rev="$(pin fdt)"
mapfile -t fdt_patches < <(patch_files fdt)
if [ ! -d "$FDT" ]; then
    git init -q "$FDT"
    git -C "$FDT" fetch -q --depth 1 https://github.com/repnop/fdt.git "$fdt_rev"
    git -C "$FDT" checkout -q FETCH_HEAD
    git -C "$FDT" apply "${fdt_patches[@]}"
fi
[ "$(git -C "$FDT" rev-parse HEAD)" = "$fdt_rev" ] || {
    echo "$FDT is not at $fdt_rev from $LOCK: remove it to clone it again" >&2
    exit 1
}
check_drift fdt "$FDT" "${fdt_patches[@]}"

# Pin a core recipe, copy its patches next to it and check its source.
# The cookbook does not notice changed patches in an already fetched recipe:
# drop its source and build when the patch set changed.
prepare_recipe() {
    local name="$1" recipe="$REDOX/recipes/core/$1"
    local -a files
    mapfile -t files < <(patch_files "$name")
    local stamp="$recipe/.redox-rpi-patches" sum patch
    sum="$(for patch in "${files[@]}"; do recipe_patch_name "$patch"; cat "$patch"; done | sha256sum | cut -d' ' -f1)"
    if [ "$(cat "$stamp" 2>/dev/null)" != "$sum" ]; then
        rm -rf "$recipe/source" "$recipe/target"
        echo "$sum" > "$stamp"
    fi
    check_drift "$name" "$recipe/source" "${files[@]}"
    recipe_patches=""
    for patch in "${files[@]}"; do
        cp "$patch" "$recipe/$(recipe_patch_name "$patch")"
        recipe_patches="$recipe_patches\"$(recipe_patch_name "$patch")\", "
    done
}

# Kernel recipe: apply our kernel patches and build against the patched fdt.
# Rewritten on every run so changes to the patches are picked up.
git -C "$REDOX" checkout -q -- recipes/core/kernel
prepare_recipe kernel
cat > "$KERNEL_RECIPE/recipe.toml" <<EOF
[source]
git = "https://gitlab.redox-os.org/redox-os/kernel.git"
rev = "$(pin kernel)"
patches = [ $recipe_patches]

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

# Other recipes: pin them and add our patches to their [source] section.
for name in bootloader base relibc; do
    git -C "$REDOX" checkout -q -- "recipes/core/$name"
    prepare_recipe "$name"
    sed -i "/^git = /a rev = \"$(pin "$name")\"\npatches = [ $recipe_patches]" \
        "$REDOX/recipes/core/$name/recipe.toml"
done

# Our own packages, built from this repository.
for tool in cputop devshd i2c netget reboot sigregs spi timerd timesync; do
    # sigregs is linked statically (the cargo template links dynamically):
    # dynamically linked, it fails at start on a real Pi before it can test
    # the signal entry (docs/tasks.md, T12). It must link the libc.a of our
    # relibc recipe, not the toolchain's. Cargo does not see a new libc.a,
    # so its fingerprint goes first: otherwise a build with other relibc
    # patches keeps the old sigregs.
    build='template = "cargo"'
    if [ "$tool" = sigregs ]; then
        build='template = "custom"
dependencies = ["relibc"]
script = """
export RUSTFLAGS="${RUSTFLAGS} -L native=${COOKBOOK_SYSROOT}/lib"
rm -rf "${COOKBOOK_BUILD}"/target/*/release/.fingerprint/sigregs-*
COOKBOOK_CARGO_PATH=. cookbook_cargo
"""'
    fi
    mkdir -p "$REDOX/recipes/other/$tool"
    cat > "$REDOX/recipes/other/$tool/recipe.toml" <<EOF
[source]
path = "$ROOT/tools/$tool"

[build]
$build
EOF
done

# Uncommitted changes to anything that goes into the image.
repo_changes="$(git -C "$ROOT" status --porcelain -- patches config scripts tools)"
if [ -n "$repo_changes" ]; then
    echo "uncommitted changes:" >&2
    echo "$repo_changes" >&2
    {
        echo "### redox-rpi working tree"
        echo "$repo_changes"
        git -C "$ROOT" diff HEAD -- patches config scripts tools
    } >> "$drift_report"
    dirty=1
fi
if [ "$dirty" = 1 ] && [ "$allow_dirty" = 0 ]; then
    echo "Commit the changes (direct edits in a recipe source as a patch), or" >&2
    echo "pass --allow-dirty to build anyway and record them in the manifest." >&2
    exit 1
fi

# Build ID: the redox-rpi version, a short hash of the experiments and whether
# the build has unrecorded changes. Shown above the login prompt and in
# /etc/redox-rpi-release. The same set of experiments, in any order, gives the
# same hash; their names are in the manifest and on the line below the ID.
version="${VERSION:-$(git -C "$ROOT" describe --tags --always)}"
build_id="$version"
exp_list=""
if [ "${#experiments[@]}" -gt 0 ]; then
    exp_list="$(printf '%s\n' "${experiments[@]}" | sort -u | paste -sd ' ' -)"
    build_id="$build_id+x$(printf '%s\n' $exp_list | sha256sum | cut -c1-4)"
fi
dirty_sum=""
if [ "$dirty" = 1 ]; then
    dirty_sum="$(sha256sum < "$drift_report" | cut -c1-12)"
    build_id="$build_id-dirty.$dirty_sum"
fi
build_date="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
# A release or candidate name goes in front of the build ID on the console.
title="$build_id"
[ -n "$release" ] && title="$release ($build_id)"

# Manifest: everything that decides what is in the image.
manifest_head="$(mktemp)"
{
    echo "# redox-rpi build manifest, written by scripts/build-image.sh"
    echo "build_id = \"$build_id\""
    [ -n "$release" ] && echo "release = \"$release\""
    echo "image = \"$image_name\""
    echo "config = \"config/$(basename "$CONFIG")\""
    echo "built = \"$build_date\""
    echo "commit = \"$(git -C "$ROOT" rev-parse HEAD)\""
    echo "dirty = \"${dirty_sum:-no}\""
    printf 'experiments = ['
    for exp in "${experiments[@]}"; do printf ' "%s",' "$exp"; done
    echo ' ]'
    echo
    echo "[upstream]"
    awk '!/^#/ && NF == 2 { printf "%s = \"%s\"\n", $1, $2 }' "$LOCK"
    echo
    echo "[patches]"
    for name in fdt kernel bootloader base relibc; do
        patch_files "$name" | while read -r patch; do
            echo "\"${patch#"$ROOT/patches/"}\" = \"$(sha256sum < "$patch" | cut -c1-16)\""
        done
    done
    for exp in "${experiments[@]}"; do
        conf="$ROOT/patches/experiments/$exp/config.toml"
        [ -f "$conf" ] && echo "\"experiments/$exp/config.toml\" = \"$(sha256sum < "$conf" | cut -c1-16)\""
    done
} > "$manifest_head"

mkdir -p "$ROOT/build"
cat > "$ROOT/build/redox-rpi-version.toml" <<EOF
# Generated by scripts/build-image.sh

[[files]]
path = "/etc/redox-rpi-release"
data = """
NAME="redox-rpi, for the Redox OS operating system"
VERSION="$version"
BUILD_ID="$build_id"
${release:+RELEASE=\"$release\"
}IMAGE="$image_name"
BUILD="$build_date"
COMMIT="$(git -C "$ROOT" rev-parse --short HEAD)"
EXPERIMENTS="$exp_list"
URL="https://github.com/redox-rpi/redox-rpi"
"""

[[files]]
path = "/etc/redox-rpi-manifest"
data = """
$(cat "$manifest_head")
"""

[[files]]
path = "/etc/issue"
data = """
########## Redox OS ##########
# Login with the following:  #
# \`user\`                     #
# \`root\`:\`password\`          #
##############################
redox-rpi $title ($image_name, $build_date)
${exp_list:+experiments: $exp_list
}
"""
EOF

# Experiments may add packages and files with a config.toml. A generated
# config includes them after the image's own config, so they win.
fs_config="$CONFIG"
exp_configs=()
for exp in "${experiments[@]}"; do
    conf="$ROOT/patches/experiments/$exp/config.toml"
    [ -f "$conf" ] && exp_configs+=("$conf")
done
if [ ${#exp_configs[@]} -gt 0 ]; then
    fs_config="$ROOT/build/experiments-$CONFIG_NAME.toml"
    {
        echo "# Generated by scripts/build-image.sh"
        printf 'include = ['
        printf ' "%s",' "$CONFIG" "${exp_configs[@]}"
        echo ' ]'
    } > "$fs_config"
fi

cd "$REDOX"
# make only rebuilds packages when repo.tag is older than what it tracks,
# which does not include our patches. The cookbook itself skips unchanged
# recipes, so letting it check every time is cheap.
rm -f "build/aarch64/$CONFIG_NAME/repo.tag"
# CI disables the cookbook's terminal UI, which needs a real terminal.
CI="${CI:-1}" make "${1:-all}" \
    PODMAN_BUILD=0 \
    ARCH=aarch64 \
    BOARD=raspi3bp \
    CONFIG_NAME="$CONFIG_NAME" \
    REPO_BINARY=1 \
    FILESYSTEM_CONFIG="$fs_config"

# The full manifest next to the image: the build inputs, hashes of the main
# outputs and the packages the image was installed from.
image="$REDOX/build/aarch64/$CONFIG_NAME/harddrive.img"
[ -f "$image" ] || exit 0
stage_file() {
    find "$REDOX/recipes/core/$1/target" -path "*/stage/$2" 2>/dev/null | head -1
}
mkdir -p "$MANIFESTS"
manifest="$MANIFESTS/$image_name-$build_id.toml"
{
    cat "$manifest_head"
    echo
    echo "[outputs]"
    echo "\"harddrive.img\" = \"$(sha256sum < "$image" | cut -d' ' -f1)\""
    for out in kernel:usr/lib/boot/kernel relibc:usr/lib/libc.so \
               bootloader:usr/lib/boot/bootloader.efi bootloader:usr/lib/boot/bootloader-live.efi; do
        file="$(stage_file "${out%%:*}" "${out#*:}")"
        [ -n "$file" ] && echo "\"/${out#*:}\" = \"$(sha256sum < "$file" | cut -d' ' -f1)\""
    done
    libc="$(stage_file relibc usr/lib/libc.so)"
    [ -n "$libc" ] && echo "libc_build_id = \"$(readelf -n "$libc" | awk '/Build ID/ { print $3 }')\""
    echo
    echo "# Packages of the repository the image was installed from:"
    echo "# version, upstream source commit, package hash."
    echo "[packages]"
    for toml in "$REDOX"/repo/aarch64-unknown-redox/*.toml; do
        [ "$(basename "$toml")" = repo.toml ] && continue
        awk -v name="$(basename "$toml" .toml)" -F' = ' '
            $1 == "version" { gsub(/"/, "", $2); v = $2 }
            $1 == "source_identifier" { gsub(/"/, "", $2); s = substr($2, 1, 12) }
            $1 == "blake3" { gsub(/"/, "", $2); b = substr($2, 1, 16) }
            END { printf "%s = \"%s %s %s\"\n", name, v, s, b }' "$toml"
    done
} > "$manifest"
if [ "$dirty" = 1 ]; then
    cp "$drift_report" "${manifest%.toml}.diff"
fi
ln -sf "$(basename "$manifest")" "$MANIFESTS/$image_name-latest.toml"
rm -f "$manifest_head"
echo "build $build_id, manifest $manifest"
