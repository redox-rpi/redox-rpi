#!/usr/bin/env bash
# Run relibc's test suite in QEMU and compare its failures with the list of
# known failures (#13), so that a new failure is flagged and a fixed test is
# noticed.
#
# Builds the network-boot image with the experiment relibc-tests (relibc's
# tests/, linked with the relibc of this build), boots it in QEMU's raspi3b
# machine and runs /usr/share/relibc-tests/run.ion on the serial console.
# Needs no Pi. Run from the build host, in your own clone:
#
#   scripts/relibc-tests.sh [--experiment <name>]... [--image <file>]
#                           [--expected <file>] [--issue]
#
#   --experiment  build with this experiment as well (repeatable)
#   --image       test this image instead of building one; it must have been
#                 built with --experiment relibc-tests
#   --expected    known failures (default config/relibc-tests-expected.txt)
#   --issue       if the result differs from the list, open or comment on the
#                 issue "Nightly: relibc tests differ from the expected
#                 failures" (token in ~/.config/redox-rpi/github-token)
#
# Exit status: 0 no new failure, 1 new failure, 2 the suite did not finish.
# Tests on the list that passed are reported as fixed but do not fail the
# run: remove them from the list. The result is written to
# build/relibc-tests/<build ID>.txt, the serial log next to it.
#
# Nightly: config/redox-relibc-tests.service checks out origin/main in
# ~/redox-rpi-relibc-tests and runs this script with --issue.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO=redox-rpi/redox-rpi
TOKEN_FILE="$HOME/.config/redox-rpi/github-token"
ISSUE_TITLE="Nightly: relibc tests differ from the expected failures"
# The suite took 404 s in QEMU on 2026-10-29; boot about 70 s.
TIMEOUT=2400

image=""
expected="$ROOT/config/relibc-tests-expected.txt"
issue=0
experiments=(--experiment relibc-tests)
while [ $# -gt 0 ]; do
    case "$1" in
        --experiment) experiments+=(--experiment "$2"); shift 2 ;;
        --image) image="$(readlink -f "$2")"; shift 2 ;;
        --expected) expected="$(readlink -f "$2")"; shift 2 ;;
        --issue) issue=1; shift ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done

log() { echo "$(date -u +%H:%M:%S) $*"; }

cd "$ROOT"
out="$ROOT/build/relibc-tests"
mkdir -p "$out"

if [ -z "$image" ]; then
    log "building with: ${experiments[*]}"
    # Build and copy under one lock, so that no other build replaces the
    # image in between.
    if ! scripts/with-build-lock.sh bash -c '
        scripts/build-image.sh "$@" &&
        cp upstream/redox/build/aarch64/minimal/harddrive.img build/relibc-tests/image.img &&
        cp "$(readlink -f build/manifests/netboot-latest.toml)" build/relibc-tests/image.toml
    ' build "${experiments[@]}" > "$out/build.log" 2>&1; then
        tail -n 20 "$out/build.log"
        log "build failed, log in $out/build.log"
        exit 2
    fi
    image="$out/image.img"
    build_id="$(sed -n 's/^build_id = "\(.*\)"$/\1/p' "$out/image.toml")"
else
    build_id="$(basename "$image" .img)"
fi
log "testing $build_id"

serial="$out/$build_id.log"
scripts/with-build-lock.sh scripts/qemu-raspi3b.sh --image "$image" --timeout "$TIMEOUT" \
    --run 'ion /usr/share/relibc-tests/run.ion' > "$out/qemu.out" 2>&1 < /dev/null || true
cp images/qemu-test.log "$serial"

result="$out/$build_id.txt"
status=0
python3 scripts/relibc-tests-compare.py "$serial" "$expected" "$build_id" > "$result" || status=$?
cat "$result"
log "result in $result, serial log in $serial"

if [ "$issue" = 1 ] && { [ "$status" != 0 ] || grep -q '^FIXED ' "$result"; }; then
    github() {
        curl -fsS -H @<(printf 'Authorization: Bearer %s\n' "$(tr -d '[:space:]' < "$TOKEN_FILE")") \
            -H "Accept: application/vnd.github+json" "$@"
    }
    body="relibc test suite in QEMU, build \`$build_id\` ($(git rev-parse --short HEAD)), $(date -u +%Y-%m-%d):

\`\`\`text
$(grep -E '^(NEW|FIXED|SUMMARY|ERROR) ' "$result")
\`\`\`

Known failures: \`config/relibc-tests-expected.txt\`. Serial log: \`$serial\` on the build host."
    number="$(github "https://api.github.com/repos/$REPO/issues?state=open&per_page=100" |
        python3 -c 'import json, sys; t = sys.argv[1]
print(next((str(i["number"]) for i in json.load(sys.stdin) if i["title"] == t and "pull_request" not in i), ""))' "$ISSUE_TITLE")"
    if [ -n "$number" ]; then
        payload="$(python3 -c 'import json, sys; print(json.dumps({"body": sys.argv[1]}))' "$body")"
        github -X POST "https://api.github.com/repos/$REPO/issues/$number/comments" -d "$payload" >/dev/null
        log "commented on issue #$number"
    else
        payload="$(python3 -c 'import json, sys; print(json.dumps({"title": sys.argv[1], "body": sys.argv[2], "labels": ["topic:ci"]}))' "$ISSUE_TITLE" "$body")"
        number="$(github -X POST "https://api.github.com/repos/$REPO/issues" -d "$payload" |
            python3 -c 'import json, sys; print(json.load(sys.stdin)["number"])')"
        log "opened issue #$number"
    fi
fi
exit "$status"
