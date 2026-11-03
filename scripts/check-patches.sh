#!/usr/bin/env bash
# Check that this repository's patches apply to the upstream sources: the
# commits pinned in config/upstream.lock, or with --latest the newest commit
# of each upstream branch. Needs git only; builds nothing.
#
# Usage:
#   scripts/check-patches.sh [--latest] [--work <dir>]
#
#   --latest      check against upstream HEAD instead of the pinned commits
#   --work <dir>  where the sources are fetched (default: build/check-patches)
#
# For each component it applies the default patches in order, then each
# experiment on top of them (with the experiments it needs first). It prints
# one line per patch set and a Markdown summary to $GITHUB_STEP_SUMMARY when
# that is set. Exit status: 1 if a default patch set does not apply, 0
# otherwise; experiments that do not apply are reported but do not fail.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOCK="$ROOT/config/upstream.lock"
work="$ROOT/build/check-patches"
latest=0
while [ $# -gt 0 ]; do
    case "$1" in
        --latest) latest=1; shift ;;
        --work) work="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done

# component   repository
repos="kernel     https://gitlab.redox-os.org/redox-os/kernel.git
relibc     https://gitlab.redox-os.org/redox-os/relibc.git
base       https://gitlab.redox-os.org/redox-os/base.git
bootloader https://gitlab.redox-os.org/redox-os/bootloader.git
fdt        https://github.com/repnop/fdt.git
netutils   https://gitlab.redox-os.org/redox-os/netutils.git"

# Experiments that apply only after another one (see their READMEs).
needs() {
    case "$1" in
        sdhost-boot) echo sdhost-driver ;;
        sdhost-stats) echo sdhost-driver sdhost-boot ;;
        dwc2-interrupts) echo rtl8152 ;;
    esac
}

shopt -s nullglob

pinned() { awk -v n="$1" '$1 == n { print $2 }' "$LOCK"; }

summary=()
failed=0
mkdir -p "$work"
while read -r name url; do
    src="$work/$name"
    rm -rf "$src"
    git init -q "$src"
    if [ "$latest" = 1 ]; then
        git -C "$src" fetch -q --depth 1 "$url" HEAD
    else
        git -C "$src" fetch -q --depth 1 "$url" "$(pinned "$name")"
    fi
    git -C "$src" checkout -q FETCH_HEAD
    rev="$(git -C "$src" rev-parse --short HEAD)"

    # Apply patch files to the index; on failure, name the first one that
    # does not apply.
    apply() {
        local p
        for p in "$@"; do
            git -C "$src" apply --index "$p" 2>/dev/null || { echo "${p#"$ROOT/patches/"}"; return 1; }
        done
    }

    defaults=("$ROOT"/patches/"$name"/*.patch)
    if bad="$(apply "${defaults[@]}")"; then
        echo "ok    $name@$rev: ${#defaults[@]} default patches"
        summary+=("| $name | \`$rev\` | default (${#defaults[@]}) | ok |")
    else
        echo "FAIL  $name@$rev: default patches, first failing: $bad"
        summary+=("| $name | \`$rev\` | default | **fails at \`$bad\`** |")
        failed=1
        continue
    fi
    base_tree="$(git -C "$src" write-tree)"

    for dir in "$ROOT"/patches/experiments/*/"$name"; do
        patches=("$dir"/*.patch)
        [ ${#patches[@]} -gt 0 ] || continue
        exp="$(basename "$(dirname "$dir")")"
        git -C "$src" read-tree --reset -u "$base_tree"
        files=()
        for dep in $(needs "$exp"); do
            files+=("$ROOT"/patches/experiments/"$dep"/"$name"/*.patch)
        done
        files+=("${patches[@]}")
        if bad="$(apply "${files[@]}")"; then
            echo "ok    $name@$rev: experiment $exp"
            summary+=("| $name | \`$rev\` | experiment \`$exp\` | ok |")
        else
            echo "warn  $name@$rev: experiment $exp, first failing: $bad"
            summary+=("| $name | \`$rev\` | experiment \`$exp\` | fails at \`$bad\` |")
        fi
    done
done <<< "$repos"

if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    {
        if [ "$latest" = 1 ]; then echo "### Patches on the latest upstream"
        else echo "### Patches on the pinned upstream commits"; fi
        echo
        echo "| Component | Commit | Patches | Result |"
        echo "|---|---|---|---|"
        printf '%s\n' "${summary[@]}"
    } >> "$GITHUB_STEP_SUMMARY"
fi
exit "$failed"
