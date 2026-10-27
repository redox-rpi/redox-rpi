#!/usr/bin/env bash
# Check out relibc as the image build uses it: the commit from
# config/upstream.lock with patches/relibc/ and, optionally, relibc
# experiments applied as commits. For working on relibc outside the image
# build, and for CI (.github/workflows/relibc.yml).
#
# Usage:
#   scripts/relibc-checkout.sh <dir> [--experiment <name>]...
#
# Fetches from github.com/redox-rpi/relibc, which has the pinned commit
# (branch `upstream`); the submodules come from gitlab.redox-os.org.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO=https://github.com/redox-rpi/relibc.git

dir=""
experiments=()
while [ $# -gt 0 ]; do
    case "$1" in
        --experiment)
            [ -d "$ROOT/patches/experiments/$2/relibc" ] || { echo "no relibc patches in patches/experiments/$2" >&2; exit 1; }
            experiments+=("$2"); shift 2 ;;
        -*) echo "unknown argument: $1" >&2; exit 1 ;;
        *) dir="$1"; shift ;;
    esac
done
[ -n "$dir" ] || { echo "usage: $0 <dir> [--experiment <name>]..." >&2; exit 1; }
[ -e "$dir" ] && { echo "$dir exists" >&2; exit 1; }

rev="$(awk '$1 == "relibc" { print $2 }' "$ROOT/config/upstream.lock")"
git init -q "$dir"
git -C "$dir" fetch -q --depth 1 "$REPO" "$rev"
git -C "$dir" checkout -q FETCH_HEAD
git -C "$dir" submodule -q update --init --depth 1

am() {
    git -C "$dir" -c user.name=redox-rpi -c user.email=redox-rpi@localhost am -q "$@"
}
am "$ROOT"/patches/relibc/*.patch
for exp in "${experiments[@]}"; do
    am "$ROOT/patches/experiments/$exp/relibc"/*.patch
done
git -C "$dir" log --oneline "$rev"..HEAD
