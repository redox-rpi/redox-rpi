#!/usr/bin/env bash
# Nightly stable image: build origin/main as the network-boot image, test it
# on the real Pi and make it the stable image if it passes
# (lab.md in redox-rpi/pi-tests, "Stable image").
#
# Runs on the build host from its own clone (default ~/redox-rpi-stable),
# started by the systemd user timer redox-stable.timer, or by hand:
#
#   scripts/update-stable.sh [--force] [--taken <N>] [--no-push] [--ref <ref>]
#
#   --force     build and test even if nothing that goes into the image
#               changed since the current stable image
#   --taken <N> for a run by hand: row N of the test queue is this run
#               and marked "on the Pi"; any other test on the Pi still stops
#               the script
#   --no-push   do not commit, push or open an issue; print what would be done
#   --ref       build this ref instead of origin/main (for testing the script)
#
# Steps:
#   1. Exit if patches/, config/, scripts/ and tools/ on origin/main are the
#      same as in the commit of the current stable image.
#   2. Exit if a row in the test queue (queue.md on main of
#      redox-rpi/pi-tests) is marked "on the Pi".
#   3. Build with the options in config/stable-image.conf.
#   4. Stage the image on TFTP, reset the Pi, wait for the login prompt on the
#      serial console and run scripts/pi-test.py.
#   5. Pass: back up the TFTP files to /srv/tftp/backup-<build ID>/, point
#      /srv/tftp/stable at it, publish its packages to the pkg repository
#      /srv/tftp/pkg (scripts/publish-pkg-repo.sh; a build with a test-only
#      experiment is built again with --no-test-experiments for the
#      packages), and in redox-rpi/pi-tests
#      update "Stable image" in lab.md, write the report to
#      reports/stable-<date>.md, commit and push to its main.
#   6. Fail: stage the previous stable image again, reset the Pi, and open or
#      update the issue "Nightly: main fails on the Pi".
#   After 1 (nothing new) or 5: start redox-visit.service, the images in
#   config/visit-images.conf (scripts/build-visit-images.sh).
#   Every night (1, 5 and 6): scripts/clippy-redox.py (our base drivers,
#   report-only) and scripts/quality-kpis.py at the checked-out
#   commit writes kpis.md and kpis.json into redox-rpi/pi-tests; on a pass
#   its --summary goes into the day's report too. Skipped while the script
#   does not exist; never fails this script.
#
# GitHub access: a token in ~/.config/redox-rpi/github-token (mode 600) with
# write access to issues of redox-rpi/redox-rpi and to contents of
# redox-rpi/pi-tests. The pi-tests clone is $PI_TESTS_CLONE (default
# ~/pi-tests-stable); the script clones it if it is missing.

set -euo pipefail

ROOT="${UPDATE_STABLE_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
# The script checks out origin/main in its own clone, which may change this
# file while bash reads it: run from a copy.
if [ -z "${UPDATE_STABLE_ROOT:-}" ]; then
    copy="$(mktemp)"
    cp "$0" "$copy"
    UPDATE_STABLE_ROOT="$ROOT" exec bash "$copy" "$@"
fi
trap 'rm -f "$0"' EXIT
REPO=redox-rpi/redox-rpi
TESTS_REPO=redox-rpi/pi-tests
TESTS="${PI_TESTS_CLONE:-$HOME/pi-tests-stable}"
TFTP=/srv/tftp
STABLE_LINK="$TFTP/stable"
SERVER=192.168.178.120
PI=192.168.178.148
SERIAL_DEV=/dev/ttyUSB0
SERIAL_LOG="$HOME/redox-rpi/logs/serial.log"
TOKEN_FILE="$HOME/.config/redox-rpi/github-token"
ISSUE_TITLE="Nightly: main fails on the Pi"
# Seconds from the reset to "redox-rpi: network boot", and to the login prompt
# (U-Boot falls back to TFTP when HTTP fails, which takes about 15 minutes).
RESET_TIMEOUT=120
BOOT_TIMEOUT=1500

args=("$@")
force=0
push=1
ref=origin/main
taken=""
while [ $# -gt 0 ]; do
    case "$1" in
        --force) force=1; shift ;;
        --no-push) push=0; shift ;;
        --ref) ref="$2"; shift 2 ;;
        --taken) taken="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done

log() { echo "$(date -u +%H:%M:%S) $*"; }

# Start the visit images (scripts/build-visit-images.sh) in their own unit,
# which outlives this one; they build only on top of a stable image that
# passed. Never fails this script.
start_visit_images() {
    [ "$push" = 1 ] || { log "--no-push: not starting the visit images"; return 0; }
    if systemctl --user cat redox-visit.service > /dev/null 2>&1; then
        systemctl --user start --no-block redox-visit.service &&
            log "started redox-visit.service (status in $TFTP/visit/status.txt)" ||
            log "could not start redox-visit.service"
    else
        log "redox-visit.service not installed, no visit images"
    fi
    return 0
}

cd "$ROOT"
git fetch -q --tags --prune origin
git checkout -q --detach "$ref"
# The service copies this script from the clone before the fetch, so it may
# be yesterday's version: run the one just checked out instead, once.
if [ -z "${UPDATE_STABLE_REEXEC:-}" ] && ! cmp -s "$0" "$ROOT/scripts/update-stable.sh"; then
    log "scripts/update-stable.sh changed in $ref, running the new version"
    copy="$(mktemp)"
    cp "$ROOT/scripts/update-stable.sh" "$copy"
    rm -f "$0"
    UPDATE_STABLE_REEXEC=1 exec bash "$copy" "${args[@]}"
fi
head="$(git rev-parse HEAD)"
log "$ref is $(git describe --tags) ($head)"

# Quality KPIs (Quality's scripts/quality-kpis.py, PR #262; interface agreed
# 2026-11-10): reads GitHub and git only, writes $kpis_dir. Never fails.
kpis_dir=""
run_kpis() {
    [ -x scripts/quality-kpis.py ] || { log "no scripts/quality-kpis.py at $ref, no KPIs"; return 0; }
    kpis_dir="$(mktemp -d)"
    local clippy=()
    # clippy on our base drivers (#272, scripts/clippy-redox.py): base at
    # this commit's config/upstream.lock with patches/base, in its own work
    # directory, so it matches HEAD whether or not tonight builds.
    rm -f build/clippy-redox.json
    if [ -x scripts/clippy-redox.py ]; then
        if timeout 900 scripts/clippy-redox.py --work build/clippy-redox \
            --json build/clippy-redox.json > "$kpis_dir/clippy.log" 2>&1; then
            log "clippy-redox: $(tail -n 1 "$kpis_dir/clippy.log")"
        else
            log "clippy-redox failed: $(tail -n 1 "$kpis_dir/clippy.log")"
        fi
    fi
    [ -s build/clippy-redox.json ] && clippy=(--clippy build/clippy-redox.json)
    if GH_TOKEN="$(tr -d '[:space:]' < "$TOKEN_FILE")" timeout 600 scripts/quality-kpis.py --rev HEAD \
        --out "$kpis_dir/kpis.md" --json "$kpis_dir/kpis.json" --summary "${clippy[@]}" \
        > "$kpis_dir/summary.md" 2> "$kpis_dir/log.txt"; then
        log "quality KPIs collected"
    else
        log "quality-kpis.py failed: $(tail -n 1 "$kpis_dir/log.txt")"
        rm -rf "$kpis_dir"
        kpis_dir=""
    fi
    return 0
}

# Copy the KPI page into the pi-tests clone (checked out on origin/main) and
# stage it.
stage_kpis() {
    [ -n "$kpis_dir" ] || return 0
    cp "$kpis_dir/kpis.md" "$kpis_dir/kpis.json" "$TESTS/"
    git -C "$TESTS" add kpis.md kpis.json
}

# Nights without a report (nothing new, or a failed build): the KPI page on
# its own commit. Never fails.
push_kpis_only() {
    [ -n "$kpis_dir" ] || return 0
    [ -d "$TESTS/.git" ] || git clone -q "https://github.com/$TESTS_REPO" "$TESTS" || return 0
    git -C "$TESTS" fetch -q origin && git -C "$TESTS" checkout -q --detach origin/main || return 0
    stage_kpis
    git -C "$TESTS" diff --cached --quiet && { log "KPIs unchanged"; return 0; }
    git -C "$TESTS" -c user.name="redox-rpi nightly" -c user.email="zehrer-xyz@users.noreply.github.com" \
        commit -q -m "Nightly: quality KPIs $(date -u +%Y-%m-%d)" || return 0
    if [ "$push" = 0 ]; then
        log "--no-push: KPI commit $(git -C "$TESTS" rev-parse --short HEAD) in $TESTS not pushed"
    elif git_push "$TESTS" ||
        { git -C "$TESTS" fetch -q origin && git -C "$TESTS" rebase -q origin/main && git_push "$TESTS"; }; then
        log "pushed the quality KPIs to $TESTS_REPO main"
    else
        log "could not push the quality KPIs to $TESTS_REPO"
    fi
    return 0
}

run_kpis

# 1. Anything new for the image?
stable_commit=""
[ -f "$STABLE_LINK/manifest.toml" ] &&
    stable_commit="$(sed -n 's/^commit = "\(.*\)"$/\1/p' "$STABLE_LINK/manifest.toml")"
stable_id="$(sed -n 's/^build_id = "\(.*\)"$/\1/p' "$STABLE_LINK/manifest.toml" 2>/dev/null || true)"
log "stable image: ${stable_id:-none} (${stable_commit:-no commit})"
if [ "$force" = 0 ] && [ -n "$stable_commit" ] &&
    git cat-file -e "$stable_commit" 2>/dev/null &&
    git diff --quiet "$stable_commit" HEAD -- patches config scripts tools; then
    log "no change to patches/, config/, scripts/ or tools/ since the stable image, nothing to do"
    push_kpis_only
    start_visit_images
    exit 0
fi

# 2. Is the Pi free? The queue is queue.md on main of pi-tests, whatever --ref is.
[ -d "$TESTS/.git" ] || git clone -q "https://github.com/$TESTS_REPO" "$TESTS"
queue="$(PI_TESTS_CLONE="$TESTS" scripts/pi-tests-queue.sh)" ||
    { log "cannot read the test queue from $TESTS_REPO, not touching the Pi"; exit 1; }
on_pi="$(grep '^| .*\*\*on the Pi\*\*' <<< "$queue" || true)"
[ -n "$taken" ] && on_pi="$(grep -v "^| $taken |" <<< "$on_pi" || true)"
if [ -n "$on_pi" ]; then
    log "a test is on the Pi (queue.md in $TESTS_REPO), not touching it"
    echo "$on_pi" | cut -c1-200
    exit 0
fi
if [ -n "$taken" ] && ! grep -q "^| $taken |.*\*\*on the Pi\*\*" <<< "$queue"; then
    log "test $taken is not marked on the Pi in queue.md of $TESTS_REPO; take the Pi first"
    exit 1
fi

# 3. Build.
# One option per word; the file may end without a newline.
mapfile -t options < <(grep -v '^#' config/stable-image.conf | tr -s ' \t' '\n' | grep -v '^$')
log "building with: ${options[*]}"
scripts/build-image.sh "${options[@]}"
image="$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img"
manifest="$(readlink -f build/manifests/netboot-latest.toml)"
build_id="$(sed -n 's/^build_id = "\(.*\)"$/\1/p' "$manifest")"
log "built $build_id"

# Reset the Pi and wait for the login prompt of the new boot.
reset_pi() {
    local start
    start="$(stat -c %s "$SERIAL_LOG")"
    log "resetting the Pi through devshd"
    timeout 30 python3 scripts/devsh.py "$PI" 2323 'echo reset > /scheme/sys/kstop' >/dev/null 2>&1 || true
    if ! wait_serial "$start" 'redox-rpi: network boot' "$RESET_TIMEOUT"; then
        # devshd is not answering: log in on the serial console instead. If a
        # shell is already open there, "root" and "password" only fail.
        log "no reset through devshd, trying the serial console"
        start="$(stat -c %s "$SERIAL_LOG")"
        serial_type ''; sleep 2
        serial_type root; sleep 3
        serial_type password; sleep 5
        serial_type 'echo reset > /scheme/sys/kstop'
        wait_serial "$start" 'redox-rpi: network boot' "$RESET_TIMEOUT" ||
            { log "the Pi did not reset"; return 1; }
    fi
    wait_serial "$start" 'redox login:' "$BOOT_TIMEOUT" ||
        { log "no login prompt after $BOOT_TIMEOUT s"; return 1; }
    # devshd starts shortly before the prompt; give the network a moment.
    sleep 20
}

# Type a line on the serial console, one character every 0.15 s, then Enter:
# the console drops characters that come faster (#28).
serial_type() {
    local text=$1 i
    for ((i = 0; i < ${#text}; i++)); do
        printf '%s' "${text:i:1}" > "$SERIAL_DEV"
        sleep 0.15
    done
    printf '\r' > "$SERIAL_DEV"
}

# Wait until <pattern> appears in the serial log after byte <start>.
wait_serial() {
    local start=$1 pattern=$2 timeout=$3 t=0
    while [ "$t" -lt "$timeout" ]; do
        tail -c +"$((start + 1))" "$SERIAL_LOG" | tr -d '\r' | grep -aq "$pattern" && return 0
        sleep 5; t=$((t + 5))
    done
    return 1
}

stage() {
    scripts/netboot.sh --server "$SERVER" --tftp-dir "$TFTP" --image "$1" >/dev/null
}

restore_stable() {
    [ -d "$STABLE_LINK" ] || { log "no stable image to restore"; return 1; }
    log "staging the stable image $stable_id again"
    cp -p "$STABLE_LINK"/* "$TFTP/redox/"
    reset_pi || true
}

# 4. Stage and test.
report="$(mktemp)"
output="$(mktemp)"
trap 'rm -f "$report" "$output" "$0"' EXIT
stage "$image"
log "staged $build_id on $TFTP/redox"
passed=0
if reset_pi; then
    log "running scripts/pi-test.py"
    if python3 scripts/pi-test.py "$PI" --manifest "$manifest" --report "$report" \
        --note "Nightly stable image test (scripts/update-stable.sh)." 2>&1 | tee "$output"; then
        passed=1
    fi
else
    echo "FAIL boot: no login prompt on the serial console" | tee "$output"
fi

# The token never appears on a command line, in a URL or in .git/config:
# the credential helper and curl read it from the file.
# git_push <dir>: push HEAD of the clone in <dir> to its main.
git_push() {
    git -C "$1" -c credential.helper= \
        -c "credential.helper=!f() { echo username=x-access-token; printf 'password='; tr -d '[:space:]' < '$TOKEN_FILE'; echo; }; f" \
        push -q origin HEAD:main
}

github() {
    curl -fsS -H @<(printf 'Authorization: Bearer %s\n' "$(tr -d '[:space:]' < "$TOKEN_FILE")") \
        -H "Accept: application/vnd.github+json" "$@"
}

# Our pkg repository follows the stable image (#44). Never devshd, and the
# signing key stays in ~/.config/redox-rpi/pkg-key. Every non-release build
# has the test experiments (config/test-experiments.conf), and
# scripts/publish-pkg-repo.sh refuses packages from a build with a test-only
# one (netstack-watch). Then the same commit and options are built again with
# --no-test-experiments, only for the packages: the stable image on TFTP stays
# the tested one. Never fails this script.
publish_pkg_repo() {
    local plog="$backup/pkg-publish.log" exp test_only=""
    for exp in $(sed -n 's/^experiments = \[\(.*\)\]/\1/p' "$manifest" | tr -d '",'); do
        [ -f "patches/experiments/$exp/test-only" ] && test_only="$test_only $exp"
    done
    : > "$plog"
    if [ -n "$test_only" ]; then
        log "test-only experiment(s)$test_only in $build_id: building the packages again with --no-test-experiments"
        if ! scripts/build-image.sh "${options[@]}" --no-test-experiments >> "$plog" 2>&1; then
            log "the package build without test experiments failed, pkg repository not published, see $plog"
            return 0
        fi
        log "packages from $(sed -n 's/^build_id = "\(.*\)"$/\1/p' "$(readlink -f build/manifests/netboot-latest.toml)")"
    fi
    scripts/publish-pkg-repo.sh >> "$plog" 2>&1 &&
        log "published the pkg repository: $(tail -n 1 "$plog")" ||
        log "publishing the pkg repository failed, see $plog"
    return 0
}

# 5. Pass: make it the stable image.
if [ "$passed" = 1 ]; then
    backup="$TFTP/backup-$build_id"
    mkdir -p "$backup"
    cp -p "$TFTP/redox"/* "$backup/"
    ln -sfn "$(basename "$backup")" "$STABLE_LINK"
    log "$build_id is the stable image, backup in $backup"
    publish_pkg_repo
    summary="$(grep -E '^(PASS|FAIL) ' "$output" | cut -d: -f1 | paste -sd ' ' -)"
    starts="$(sed -n 's/^PASS starts: //p' "$output" | head -1)"
    # The report goes to pi-tests, on top of its newest main.
    date="$(date -u +%Y-%m-%d)"
    git -C "$TESTS" fetch -q origin
    git -C "$TESTS" checkout -q --detach origin/main
    python3 - "$build_id" "$date" "${starts:-see report}" "$TESTS/lab.md" <<'EOF'
import re, sys
build_id, date, starts, path = sys.argv[1:]
text = open(path).read()
block = (f"<!-- stable-image: written by scripts/update-stable.sh -->\n"
         f"Staged on the TFTP server whenever no test runs: `{build_id}`, tested\n"
         f"{date} with `scripts/pi-test.py` ([report](reports/stable-{date}.md), starts:\n"
         f"{starts}). A copy is in `/srv/tftp/backup-{build_id}/`, and\n"
         f"`/srv/tftp/stable` points to it.\n"
         f"<!-- /stable-image -->")
new, n = re.subn(r"<!-- stable-image.*?<!-- /stable-image -->", lambda m: block, text, flags=re.S)
if n != 1:
    sys.exit(f"no stable-image block in {path}")
open(path, "w").write(new)
EOF
    # A second run on the same day adds to that day's file.
    out="$TESTS/reports/stable-$date.md"
    { [ -s "$out" ] && echo; sed '1s/^## /# /' "$report"; } >> "$out"
    if [ -n "$kpis_dir" ] && [ -s "$kpis_dir/summary.md" ]; then
        { echo; echo "## Quality KPIs"; echo; cat "$kpis_dir/summary.md"; } >> "$out"
    fi
    stage_kpis
    git -C "$TESTS" add lab.md "reports/stable-$date.md"
    git -C "$TESTS" -c user.name="redox-rpi nightly" -c user.email="zehrer-xyz@users.noreply.github.com" \
        commit -q -m "Nightly: $build_id is the stable image

scripts/update-stable.sh built redox-rpi $ref ${head:0:7} and ran
scripts/pi-test.py on the real Pi: $summary."
    if [ "$push" = 1 ]; then
        if git_push "$TESTS" ||
            { git -C "$TESTS" fetch -q origin && git -C "$TESTS" rebase -q origin/main && git_push "$TESTS"; }; then
            log "pushed $(git -C "$TESTS" rev-parse --short HEAD) to $TESTS_REPO main"
        else
            # The image is the stable image all the same; only the record is late.
            log "could not push the report to $TESTS_REPO (token access?); commit kept in $TESTS"
        fi
    else
        log "--no-push: commit $(git -C "$TESTS" rev-parse --short HEAD) in $TESTS not pushed"
        git -C "$TESTS" show --stat HEAD | cat
    fi
    start_visit_images
    exit 0
fi

# 6. Fail: back to the stable image, and tell people.
log "$build_id failed on the Pi"
restore_stable
# The failing checks, or the end of the output if pi-test.py stopped early.
failed="$(grep -E '^FAIL ' "$output" || tail -n 4 "$output" | cut -c1-300)"
body="Build \`$build_id\` ($ref ${head:0:7}, options \`$(grep -v '^#' config/stable-image.conf | tr '\n' ' ')\`) failed on the real Pi on $(date -u +%Y-%m-%d).

Failing checks:

\`\`\`text
$failed
\`\`\`

The stable image \`$stable_id\` is staged again. Log: \`~/redox-rpi/logs/stable/\` on the build host."
if [ "$push" = 0 ]; then
    log "--no-push: would open or update the issue \"$ISSUE_TITLE\":"
    echo "$body"
    exit 1
fi
number="$(github "https://api.github.com/repos/$REPO/issues?state=open&per_page=100" |
    python3 -c 'import json, sys; t = sys.argv[1]
print(next((str(i["number"]) for i in json.load(sys.stdin) if i["title"] == t and "pull_request" not in i), ""))' "$ISSUE_TITLE")"
payload="$(python3 -c 'import json, sys; print(json.dumps({"body": sys.argv[1]}))' "$body")"
if [ -n "$number" ]; then
    github -X POST "https://api.github.com/repos/$REPO/issues/$number/comments" -d "$payload" >/dev/null
    log "commented on issue #$number"
else
    payload="$(python3 -c 'import json, sys; print(json.dumps({"title": sys.argv[1], "body": sys.argv[2], "labels": ["topic:ci", "needs:pi"]}))' "$ISSUE_TITLE" "$body")"
    number="$(github -X POST "https://api.github.com/repos/$REPO/issues" -d "$payload" |
        python3 -c 'import json, sys; print(json.load(sys.stdin)["number"])')"
    log "opened issue #$number"
fi
push_kpis_only
exit 1
