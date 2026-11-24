#!/usr/bin/env bash
# Warn when a pull request that changes only experiments (and docs) would
# close an issue: an issue closes only when its fix is in the default build
# (PMP, #692); experiment-only pull requests say "Refs #N" (#693).
#
# Usage:
#   scripts/check-pr-refs.sh <base> <head> [<body file>]
#
#   <base> <head>  the pull request's base and head commits
#   <body file>    the pull request's description (optional)
#
# Prints a GitHub warning annotation and a line in $GITHUB_STEP_SUMMARY when
# it finds one. Report only: the exit status is 0 unless the arguments are
# wrong.

set -euo pipefail

[ $# -ge 2 ] || { echo "usage: $0 <base> <head> [<body file>]" >&2; exit 2; }
base="$1" head="$2" body="${3:-}"

files="$(git diff --name-only "$base...$head")"
[ -n "$files" ] || { echo "no changed files"; exit 0; }
# Only experiments and docs, and at least one experiment.
if grep -qvE '^(patches/experiments/|docs/)|\.md$' <<< "$files" ||
    ! grep -q '^patches/experiments/' <<< "$files"; then
    echo "not an experiment-only change"
    exit 0
fi

# GitHub's closing keywords, with or without a colon.
re='\b(close[sd]?|fix(e[sd])?|resolve[sd]?):?[[:space:]]+(redox-rpi/redox-rpi)?#[0-9]+'
text="$( { [ -n "$body" ] && cat "$body"; git log --format=%B "$base..$head"; } 2>/dev/null)"
refs="$(grep -ioE "$re" <<< "$text" | sort -u | paste -sd ' ' -)" || true
if [ -z "$refs" ]; then
    echo "experiment-only change, no closing keyword"
    exit 0
fi
msg="This pull request changes only experiments, but its description or commits say \"$refs\". An issue closes only when its fix is in the default build: write \"Refs #N\" instead (#693)."
echo "::warning title=Experiment-only change closes an issue::$msg"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    { echo "### Issue references"; echo; echo "$msg"; } >> "$GITHUB_STEP_SUMMARY"
fi
