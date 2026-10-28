#!/usr/bin/env bash
# Nightly stable image: build origin/main as the network-boot image, test it
# on the real Pi and make it the stable image if it passes
# (docs/test-plan.md, "Stable image").
#
# Runs on the build host from its own clone (default ~/redox-rpi-stable),
# started by the systemd user timer redox-stable.timer, or by hand:
#
#   scripts/update-stable.sh [--force] [--no-push] [--ref <ref>]
#
#   --force     build and test even if nothing that goes into the image
#               changed since the current stable image, and even if a test
#               is marked "on the Pi" (for a run by hand that took the Pi)
#   --no-push   do not commit, push or open an issue; print what would be done
#   --ref       build this ref instead of origin/main (for testing the script)
#
# Steps:
#   1. Exit if patches/, config/, scripts/ and tools/ on origin/main are the
#      same as in the commit of the current stable image.
#   2. Exit if a test in docs/test-plan.md is marked "on the Pi".
#   3. Build with the options in config/stable-image.conf.
#   4. Stage the image on TFTP, reset the Pi, wait for the login prompt on the
#      serial console and run scripts/pi-test.py.
#   5. Pass: back up the TFTP files to /srv/tftp/backup-<build ID>/, point
#      /srv/tftp/stable at it, update "Stable image" in docs/test-plan.md,
#      append the report to docs/test-reports.md, commit and push to main.
#   6. Fail: stage the previous stable image again, reset the Pi, and open or
#      update the issue "Nightly: main fails on the Pi".
#
# GitHub access: a token with write access to contents and issues of
# redox-rpi/redox-rpi in ~/.config/redox-rpi/github-token (mode 600).

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

force=0
push=1
ref=origin/main
while [ $# -gt 0 ]; do
    case "$1" in
        --force) force=1; shift ;;
        --no-push) push=0; shift ;;
        --ref) ref="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done

log() { echo "$(date -u +%H:%M:%S) $*"; }

cd "$ROOT"
git fetch -q --tags --prune origin
git checkout -q --detach "$ref"
head="$(git rev-parse HEAD)"
log "$ref is $(git describe --tags) ($head)"

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
    exit 0
fi

# 2. Is the Pi free?
if [ "$force" = 0 ] && grep -q '\*\*on the Pi\*\*' docs/test-plan.md; then
    log "a test is on the Pi (docs/test-plan.md), not touching it; next try tomorrow"
    grep '\*\*on the Pi\*\*' docs/test-plan.md | cut -c1-200
    exit 0
fi

# 3. Build.
read -r -a options < <(grep -v '^#' config/stable-image.conf | tr '\n' ' ')
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
        printf '\r' > "$SERIAL_DEV"; sleep 2
        printf 'root\r' > "$SERIAL_DEV"; sleep 3
        printf 'password\r' > "$SERIAL_DEV"; sleep 5
        printf 'echo reset > /scheme/sys/kstop\r' > "$SERIAL_DEV"
        wait_serial "$start" 'redox-rpi: network boot' "$RESET_TIMEOUT" ||
            { log "the Pi did not reset"; return 1; }
    fi
    wait_serial "$start" 'redox login:' "$BOOT_TIMEOUT" ||
        { log "no login prompt after $BOOT_TIMEOUT s"; return 1; }
    # devshd starts shortly before the prompt; give the network a moment.
    sleep 20
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
        --note "Nightly stable image test (scripts/update-stable.sh)." | tee "$output"; then
        passed=1
    fi
else
    echo "FAIL boot: no login prompt on the serial console" | tee "$output"
fi

# The token never appears on a command line, in a URL or in .git/config:
# the credential helper and curl read it from the file.
git_push() {
    git -c credential.helper= \
        -c "credential.helper=!f() { echo username=x-access-token; printf 'password='; tr -d '[:space:]' < '$TOKEN_FILE'; echo; }; f" \
        push -q origin HEAD:main
}

github() {
    curl -fsS -H @<(printf 'Authorization: Bearer %s\n' "$(tr -d '[:space:]' < "$TOKEN_FILE")") \
        -H "Accept: application/vnd.github+json" "$@"
}

# 5. Pass: make it the stable image.
if [ "$passed" = 1 ]; then
    backup="$TFTP/backup-$build_id"
    mkdir -p "$backup"
    cp -p "$TFTP/redox"/* "$backup/"
    ln -sfn "$(basename "$backup")" "$STABLE_LINK"
    log "$build_id is the stable image, backup in $backup"
    summary="$(grep -E '^(PASS|FAIL) ' "$output" | cut -d: -f1 | paste -sd ' ' -)"
    starts="$(sed -n 's/^PASS starts: //p' "$output" | head -1)"
    # The docs change goes on top of the newest main.
    git fetch -q origin
    git checkout -q --detach origin/main
    python3 - "$build_id" "$(date -u +%Y-%m-%d)" "${starts:-see report}" <<'EOF'
import re, sys
build_id, date, starts = sys.argv[1:]
path = "docs/test-plan.md"
text = open(path).read()
block = (f"<!-- stable-image: written by scripts/update-stable.sh -->\n"
         f"Staged on the TFTP server whenever no test runs: `{build_id}`, tested\n"
         f"{date} with `scripts/pi-test.py` ([report](test-reports.md), starts:\n"
         f"{starts}). A copy is in `/srv/tftp/backup-{build_id}/`, and\n"
         f"`/srv/tftp/stable` points to it.\n"
         f"<!-- /stable-image -->")
new, n = re.subn(r"<!-- stable-image.*?<!-- /stable-image -->", lambda m: block, text, flags=re.S)
if n != 1:
    sys.exit("no stable-image block in docs/test-plan.md")
open(path, "w").write(new)
EOF
    cat "$report" >> docs/test-reports.md
    git add docs/test-plan.md docs/test-reports.md
    git -c user.name="redox-rpi nightly" -c user.email="zehrer-xyz@users.noreply.github.com" \
        commit -q -m "Nightly: $build_id is the stable image

scripts/update-stable.sh built $ref ${head:0:7} and ran
scripts/pi-test.py on the real Pi: $summary."
    if [ "$push" = 1 ]; then
        git_push || { git fetch -q origin && git rebase -q origin/main && git_push; }
        log "pushed $(git rev-parse --short HEAD) to main"
    else
        log "--no-push: commit $(git rev-parse --short HEAD) not pushed"
        git show --stat HEAD | cat
    fi
    exit 0
fi

# 6. Fail: back to the stable image, and tell people.
log "$build_id failed on the Pi"
restore_stable
failed="$(grep -E '^FAIL ' "$output" || true)"
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
exit 1
