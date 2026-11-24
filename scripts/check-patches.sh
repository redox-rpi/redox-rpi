#!/usr/bin/env bash
# Check that this repository's patches apply to the upstream sources: the
# commits pinned in config/upstream.lock, or with --latest the newest commit
# of each upstream branch. Needs git only; builds nothing.
#
# Usage:
#   scripts/check-patches.sh [--latest] [--work <dir>] [--base <rev>]
#                            [--report <file>]
#
#   --latest      check against upstream HEAD instead of the pinned commits
#   --work <dir>  where the sources are fetched (default: build/check-patches)
#   --base <rev>  also check the patches and pins of commit <rev> (a pull
#                 request's base) and fail on what this tree breaks there:
#                 an experiment that applies at <rev> but not here, a new
#                 experiment that does not apply, a new duplicate patch
#                 number (#672)
#   --report <f>  write the experiments that do not apply and are not
#                 parked to <f> as a Markdown list (the nightly's report)
#
# For each component it applies the default patches in order, then each
# experiment on top of them (with the experiments it needs first). It prints
# one line per patch set and a Markdown summary to $GITHUB_STEP_SUMMARY when
# that is set.
#
# Experiments that are knowingly broken are listed in
# patches/experiments/parked.txt, one per line: the name, then the reason.
# They never fail the check.
#
# Exit status 1 if a default patch set does not apply, or with --base if
# this tree breaks an experiment or adds a duplicate patch number; 0
# otherwise. Without --base, experiments that do not apply and duplicate
# numbers are reported as warnings.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
work="$ROOT/build/check-patches"
latest=0
base=""
report=""
while [ $# -gt 0 ]; do
    case "$1" in
        --latest) latest=1; shift ;;
        --work) work="$2"; shift 2 ;;
        --base) base="$2"; shift 2 ;;
        --report) report="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done

# component   repository
repos="kernel     https://gitlab.redox-os.org/redox-os/kernel.git
relibc     https://gitlab.redox-os.org/redox-os/relibc.git
base       https://gitlab.redox-os.org/redox-os/base.git
bootloader https://gitlab.redox-os.org/redox-os/bootloader.git
redoxfs    https://gitlab.redox-os.org/redox-os/redoxfs.git
pkgutils   https://gitlab.redox-os.org/redox-os/pkgutils.git
fdt        https://github.com/repnop/fdt.git
netutils   https://gitlab.redox-os.org/redox-os/netutils.git"

# Experiments that apply only after another one (see their READMEs).
# $1 the tree, $2 the experiment.
needs() {
    case "$2" in
        sdhost-boot) echo sdhost-driver ;;
        sdhost-stats) echo sdhost-driver sdhost-boot ;;
        dwc2-interrupts) echo rtl8152 ;;
        usb-hub-drop) echo dwc2d-netstats ;;
        dwc2d-restart) echo dwc2d-netstats usb-hub-drop ;;
        # Or the experiment's own file "needs" (the relibc CI reads it too).
        *) cat "$1/patches/experiments/$2/needs" 2>/dev/null || true ;;
    esac
}

shopt -s nullglob

mkdir -p "$work"
rm -rf "$work/src"

# The base tree: its patches and pins, from git.
base_root=""
if [ -n "$base" ]; then
    git -C "$ROOT" cat-file -e "$base^{commit}" 2>/dev/null ||
        git -C "$ROOT" fetch -q --depth 1 origin "$base"
    base_root="$work/base-tree"
    rm -rf "$base_root"
    mkdir -p "$base_root"
    git -C "$ROOT" archive "$base" patches config/upstream.lock | tar -x -C "$base_root"
fi

# Parked experiments: name -> reason.
declare -A parked=()
if [ -f "$ROOT/patches/experiments/parked.txt" ]; then
    while read -r name reason; do
        case "$name" in ''|'#'*) continue ;; esac
        parked[$name]="${reason:-no reason given}"
    done < "$ROOT/patches/experiments/parked.txt"
fi

# The upstream source of a component at a commit (or HEAD), fetched once per
# run. Prints its directory.
source_dir() {
    local name="$1" url="$2" rev="$3" src
    src="$work/src/$name-${rev:-HEAD}"
    if [ ! -e "$src/.fetched" ]; then
        rm -rf "$src"
        git init -q "$src"
        git -C "$src" fetch -q --depth 1 "$url" "${rev:-HEAD}"
        git -C "$src" checkout -q FETCH_HEAD
        touch "$src/.fetched"
        echo .fetched >> "$src/.git/info/exclude"
    fi
    echo "$src"
}

# Check one tree. $1 the tree, $2 the results file: one line
# "<component> <experiment or -> ok|fail <rev> <first failing patch or count>".
check_tree() {
    local root="$1" out="$2" name url rev src short defaults bad base_tree dir exp patches files dep
    : > "$out"
    while read -r name url; do
        if [ "$latest" = 1 ]; then
            rev=""
        else
            rev="$(awk -v n="$name" '$1 == n { print $2 }' "$root/config/upstream.lock")"
            [ -n "$rev" ] || { echo "$name - fail none no-pin-in-upstream.lock" >> "$out"; continue; }
        fi
        src="$(source_dir "$name" "$url" "$rev")"
        git -C "$src" reset -q --hard HEAD
        git -C "$src" clean -fdq
        short="$(git -C "$src" rev-parse --short HEAD)"

        # Apply patch files to the index; on failure, name the first one
        # that does not apply.
        apply() {
            local p
            for p in "$@"; do
                git -C "$src" apply --index "$p" 2>/dev/null || { echo "${p#"$root/patches/"}"; return 1; }
            done
        }

        defaults=("$root"/patches/"$name"/*.patch)
        if bad="$(apply "${defaults[@]}")"; then
            echo "$name - ok $short ${#defaults[@]}" >> "$out"
        else
            echo "$name - fail $short $bad" >> "$out"
            continue
        fi
        base_tree="$(git -C "$src" write-tree)"

        for dir in "$root"/patches/experiments/*/"$name"; do
            patches=("$dir"/*.patch)
            [ ${#patches[@]} -gt 0 ] || continue
            exp="$(basename "$(dirname "$dir")")"
            git -C "$src" read-tree --reset -u "$base_tree"
            files=()
            for dep in $(needs "$root" "$exp"); do
                files+=("$root"/patches/experiments/"$dep"/"$name"/*.patch)
            done
            files+=("${patches[@]}")
            if bad="$(apply "${files[@]}")"; then
                echo "$name $exp ok $short -" >> "$out"
            else
                echo "$name $exp fail $short $bad" >> "$out"
            fi
        done
    done <<< "$repos"
}

# Duplicate patch numbers in one directory ("0052" twice in patches/kernel):
# the order then depends on the rest of the file name. One line
# "<directory> <number>".
duplicates() {
    local root="$1" dir
    for dir in "$root"/patches/*/ "$root"/patches/experiments/*/*/; do
        find "$dir" -maxdepth 1 -name '[0-9][0-9][0-9][0-9]-*.patch' -printf '%f\n' |
            cut -c1-4 | sort | uniq -d | sed "s|^|${dir#"$root/"} |"
    done
}

head_results="$work/results-head.txt"
check_tree "$ROOT" "$head_results"
declare -A base_status=()
if [ -n "$base_root" ]; then
    check_tree "$base_root" "$work/results-base.txt"
    while read -r name exp status _; do
        base_status["$name $exp"]="$status"
    done < "$work/results-base.txt"
fi

summary=()
breaks=()
broken=()
failed=0
while read -r name exp status rev info; do
    if [ "$exp" = - ]; then
        if [ "$status" = ok ]; then
            echo "ok    $name@$rev: $info default patches"
            summary+=("| $name | \`$rev\` | default ($info) | ok |")
        else
            echo "FAIL  $name@$rev: default patches, first failing: $info"
            summary+=("| $name | \`$rev\` | default | **fails at \`$info\`** |")
            failed=1
        fi
        continue
    fi
    if [ "$status" = ok ]; then
        echo "ok    $name@$rev: experiment $exp"
        summary+=("| $name | \`$rev\` | experiment \`$exp\` | ok |")
        [ -z "${parked[$exp]:-}" ] ||
            echo "note  experiment $exp applies on $name but is in parked.txt"
        continue
    fi
    if [ -n "${parked[$exp]:-}" ]; then
        echo "park  $name@$rev: experiment $exp, first failing: $info (parked: ${parked[$exp]})"
        summary+=("| $name | \`$rev\` | experiment \`$exp\` | fails at \`$info\` (parked: ${parked[$exp]}) |")
        continue
    fi
    broken+=("- \`$exp\` on $name \`$rev\`: fails at \`$info\`")
    was="${base_status["$name $exp"]:-}"
    if [ -n "$base_root" ] && [ "$was" != fail ]; then
        if [ "$was" = ok ]; then why="applies on the base, not here"; else why="new here, does not apply"; fi
        echo "FAIL  $name@$rev: experiment $exp, first failing: $info ($why)"
        summary+=("| $name | \`$rev\` | experiment \`$exp\` | **fails at \`$info\`** ($why) |")
        breaks+=("- \`$exp\` on $name: $why, first failing \`$info\`")
        failed=1
    else
        if [ -n "$base_root" ]; then why=" (fails on the base too)"; else why=""; fi
        echo "warn  $name@$rev: experiment $exp, first failing: $info$why"
        summary+=("| $name | \`$rev\` | experiment \`$exp\` | fails at \`$info\`$why |")
    fi
done < "$head_results"

# Duplicate patch numbers: new ones fail with --base, the rest warn.
dups=()
declare -A base_dups=()
if [ -n "$base_root" ]; then
    while read -r d; do base_dups[$d]=1; done < <(duplicates "$base_root")
fi
while read -r d; do
    [ -n "$d" ] || continue
    if [ -n "$base_root" ] && [ -z "${base_dups[$d]:-}" ]; then
        echo "FAIL  duplicate patch number ${d#* } in ${d%% *} (new here)"
        dups+=("- **${d%% *}: ${d#* } twice (new here)**")
        breaks+=("- duplicate patch number ${d#* } in ${d%% *}")
        failed=1
    else
        echo "warn  duplicate patch number ${d#* } in ${d%% *}"
        dups+=("- ${d%% *}: ${d#* } twice")
    fi
done < <(duplicates "$ROOT")

if [ -n "$report" ]; then
    {
        echo "Experiments that do not apply and are not in patches/experiments/parked.txt: ${#broken[@]}"
        [ ${#broken[@]} -eq 0 ] || { echo; printf '%s\n' "${broken[@]}"; }
    } > "$report"
fi

if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    {
        if [ "$latest" = 1 ]; then echo "### Patches on the latest upstream"
        else echo "### Patches on the pinned upstream commits"; fi
        echo
        if [ ${#breaks[@]} -gt 0 ]; then
            echo "**This change breaks** (refresh in the same pull request, or park the experiment in \`patches/experiments/parked.txt\` with a reason):"
            echo
            printf '%s\n' "${breaks[@]}"
            echo
        fi
        echo "| Component | Commit | Patches | Result |"
        echo "|---|---|---|---|"
        printf '%s\n' "${summary[@]}"
        if [ ${#dups[@]} -gt 0 ]; then
            echo
            echo "Duplicate patch numbers:"
            echo
            printf '%s\n' "${dups[@]}"
        fi
    } >> "$GITHUB_STEP_SUMMARY"
fi
exit "$failed"
