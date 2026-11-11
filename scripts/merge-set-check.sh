#!/usr/bin/env bash
# Full-image QEMU check of the merge set (#260): build main plus the open
# merge-next pull requests, merged in scripts/pr-order.py order, boot the
# image in QEMU, and set the commit status "full-image-qemu" on each pull
# request's head. Report-only: nothing is merged, and no rule requires the
# status yet.
#
# Per-PR CI only checks that patches apply; it never builds base, relibc,
# redoxfs, the bootloader or the recipes, nor boots our init. This check does,
# for the pull requests together as they will land. Hardware QEMU does not
# model (real USB, audio rate, the 3B+ external abort) stays with the nightly
# and the Pi rows.
#
# Runs on the build host in its own clone (it checks out detached commits
# there), by hand or later from a timer:
#
#   scripts/merge-set-check.sh [--prs "<N> <N>..."] [--force] [--no-status]
#                              [--starts <n>]
#
#   --prs        check these pull requests (in this order) instead of the
#                merge-next set, e.g. one pull request to find the culprit
#                after a failed set
#   --force      run even if main and the pull request heads are the same as
#                in the last run
#   --no-status  do not set commit statuses; only write the report
#   --starts     program starts for scripts/pi-test.py (default 100)
#
# Steps:
#   1. Fetch main and the pull request heads; the set is the open pull
#      requests into main with the label merge-next, in pr-order.py order.
#      Exit if main and every head are unchanged since the last run.
#   2. Merge them one by one onto main (detached, no branch changes). One
#      that conflicts is left out and gets a failure status.
#   3. scripts/build-image.sh with config/stable-image.conf (the nightly's
#      options); it waits for other builds on the host (build lock).
#   4. QEMU: boot with --usb-net --data, wait for the login prompt, then
#      scripts/pi-test.py --starts <n>; then the pwm-audio QEMU check
#      (patches/experiments/pwm-audio/qemu-check.sh). Serial rules: no kernel
#      panic, no UNHANDLED EXCEPTION, no pcid or rtcd (base patch 0030).
#   5. Status "full-image-qemu" on each head: success, or failure with the
#      first failing step. The report goes to build/merge-set-check/.
#
# GitHub: gh for reading; statuses need a token with commit status write
# access to redox-rpi/redox-rpi, from $GH_TOKEN or
# ~/.config/redox-rpi/merge-check-token (never printed, never on a command
# line). One QEMU per clone: run nothing else in QEMU in this clone meanwhile.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# The checkouts below replace this file; bash reads it while it runs, so run
# from a copy.
if [ -z "${MERGE_SET_CHECK_COPY:-}" ]; then
    copy="$(mktemp)"
    cp "$0" "$copy"
    MERGE_SET_CHECK_COPY=1 MERGE_SET_CHECK_ROOT="$ROOT" exec bash "$copy" "$@"
fi
ROOT="$MERGE_SET_CHECK_ROOT"
trap 'rm -f "$0"' EXIT
cd "$ROOT"
REPO=redox-rpi/redox-rpi
CONTEXT=full-image-qemu
OUT="$ROOT/build/merge-set-check"
TOKEN_FILE="$HOME/.config/redox-rpi/merge-check-token"
DEVSH_PORT="${DEVSH_PORT:-12621}"
SSH_PORT="${SSH_PORT:-12622}"
LOGIN_TIMEOUT=900

prs=""
force=0
status=1
starts=100
while [ $# -gt 0 ]; do
    case "$1" in
        --prs) prs="$2"; shift 2 ;;
        --force) force=1; shift ;;
        --no-status) status=0; shift ;;
        --starts) starts="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done

log() { echo "$(date -u +%H:%M:%S) $*"; }
mkdir -p "$OUT"
# The trial merges (here and in pr-order.py) need an identity; the build
# host's git has none.
export GIT_AUTHOR_NAME=merge-set-check GIT_AUTHOR_EMAIL=merge-set-check@localhost
export GIT_COMMITTER_NAME=merge-set-check GIT_COMMITTER_EMAIL=merge-set-check@localhost

if [ -z "${GH_TOKEN:-}" ] && [ -r "$TOKEN_FILE" ]; then
    GH_TOKEN="$(tr -d '[:space:]' < "$TOKEN_FILE")"
    export GH_TOKEN
fi

# 1. The set.
git fetch -q origin main
if [ -z "$prs" ]; then
    set_json="$(gh pr list --repo "$REPO" --state open --base main --label merge-next \
        --json number,isDraft --limit 100)"
    candidates="$(python3 -c 'import json, sys
print(" ".join(str(p["number"]) for p in json.load(sys.stdin) if not p["isDraft"]))' <<< "$set_json")"
    # pr-order.py fetches the heads to origin/pr/N and knows the order.
    order="$(scripts/pr-order.py --repo "$REPO" --json |
        python3 -c 'import json, sys; print(" ".join(str(p["number"]) for p in json.load(sys.stdin)))')"
    for n in $order; do
        [[ " $candidates " == *" $n "* ]] && prs+="$n "
    done
    # merge-next pull requests pr-order.py left out still get checked, last.
    for n in $candidates; do
        [[ " $prs " == *" $n "* ]] || prs+="$n "
    done
fi
prs="$(echo $prs)"
declare -A head
for n in $prs; do
    git fetch -q origin "pull/$n/head:refs/remotes/origin/pr/$n" 2> /dev/null ||
        git fetch -q -f origin "pull/$n/head:refs/remotes/origin/pr/$n"
    head[$n]="$(git rev-parse "origin/pr/$n")"
done
main="$(git rev-parse origin/main)"
key="main=$main"
for n in $prs; do key+=" $n=${head[$n]}"; done
log "set: ${prs:-none} on main ${main:0:8}"
[ -n "$prs" ] || { log "no merge-next pull requests, nothing to check"; exit 0; }
if [ "$force" = 0 ] && [ -f "$OUT/last-key" ] && [ "$(cat "$OUT/last-key")" = "$key" ]; then
    log "main and the heads are unchanged since the last run"
    exit 0
fi

stamp="$(date -u +%Y%m%dT%H%M)"
report="$OUT/$stamp.md"
work="$OUT/$stamp"
mkdir -p "$work"
{
    echo "# Merge set check $stamp"
    echo
    echo "main \`${main:0:12}\`, pull requests in order: $prs"
    echo
} > "$report"

# 2. Merge.
git checkout -q --detach "$main"
merged=() conflicted=()
for n in $prs; do
    if git merge -q --no-edit -m "merge-set-check: #$n" "origin/pr/$n" > /dev/null 2>&1; then
        merged+=("$n")
    else
        git merge --abort 2> /dev/null || git reset -q --hard HEAD
        conflicted+=("$n")
        log "#$n conflicts with main plus the pull requests before it, left out"
    fi
done
echo "Merged: ${merged[*]:-none}. Conflicting, left out: ${conflicted[*]:-none}." >> "$report"

result=pass
step=""
fail() { result=fail; [ -n "$step" ] || step="$1"; echo "- FAIL $1" >> "$report"; log "FAIL $1"; }
ok() { echo "- pass $1" >> "$report"; log "pass $1"; }

if [ ${#merged[@]} -eq 0 ]; then
    fail "nothing merged"
else
    # 3. Build.
    mapfile -t options < <(grep -v '^#' config/stable-image.conf | tr -s ' \t' '\n' | grep -v '^$')
    log "building with: ${options[*]}"
    if scripts/build-image.sh "${options[@]}" > "$work/build.log" 2>&1; then
        manifest="$(readlink -f build/manifests/netboot-latest.toml)"
        build_id="$(sed -n 's/^build_id = "\(.*\)"$/\1/p' "$manifest")"
        ok "build \`$build_id\`"
        cp "$manifest" "$work/manifest.toml"
    else
        tail -n 30 "$work/build.log" > "$work/build-tail.log"
        fail "build (see build-tail.log)"
    fi
fi

# 4. QEMU.
if [ "$result" = pass ]; then
    image="$work/harddrive.img"
    cp upstream/redox/build/aarch64/minimal/harddrive.img "$image"
    console="$work/console.log"
    DEVSH_PORT="$DEVSH_PORT" SSH_PORT="$SSH_PORT" timeout 3600 \
        scripts/qemu-raspi3b.sh --usb-net --data --image "$image" > "$console" 2>&1 &
    qemu=$!
    t=0
    until tr -d '\r' < "$console" | grep -aq 'redox login:'; do
        sleep 5; t=$((t + 5))
        if [ "$t" -ge "$LOGIN_TIMEOUT" ] || ! kill -0 "$qemu" 2> /dev/null; then break; fi
    done
    if tr -d '\r' < "$console" | grep -aq 'redox login:'; then
        ok "login prompt after $t s"
        sleep 30
        if timeout 2400 python3 scripts/pi-test.py 127.0.0.1 --port "$DEVSH_PORT" --starts "$starts" \
            --manifest "$work/manifest.toml" --serial-log "$console" --report "$work/pi-test.md" \
            --note "merge-set-check $stamp" > "$work/pi-test.out" 2>&1; then
            ok "pi-test.py --starts $starts: $(grep -E '^(PASS|FAIL) ' "$work/pi-test.out" | cut -d: -f1 | paste -sd ' ' -)"
        else
            fail "pi-test.py: $(grep -E '^FAIL ' "$work/pi-test.out" | head -3 | cut -c1-120 | paste -sd ';' -)"
        fi
    else
        fail "no login prompt within $LOGIN_TIMEOUT s"
    fi
    pkill -f "hostfwd=tcp::$DEVSH_PORT-" 2> /dev/null || true
    wait "$qemu" 2> /dev/null || true
    clean="$(tr -d '\r' < "$console" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g')"
    if grep -aqiE 'kernel panic|panicked at|UNHANDLED EXCEPTION' <<< "$clean"; then
        fail "serial: $(grep -aiE 'kernel panic|panicked at|UNHANDLED EXCEPTION' <<< "$clean" | head -1 | cut -c1-100)"
    else
        ok "serial: no panic, no unhandled exception"
    fi
    if grep -aqE '^(pcid|rtcd)\b|\b(pcid|rtcd): ' <<< "$clean"; then
        fail "serial: pcid or rtcd started (base patch 0030)"
    else
        ok "serial: no pcid, no rtcd"
    fi
    audio=patches/experiments/pwm-audio/qemu-check.sh
    if [ -x "$audio" ]; then
        if timeout 1200 "$audio" "$image" > "$work/pwm-audio.out" 2>&1; then
            ok "pwm-audio qemu-check.sh"
        else
            fail "pwm-audio qemu-check.sh: $(grep '^FAIL' "$work/pwm-audio.out" | head -2 | paste -sd ';' -)"
        fi
    fi
    rm -f "$image"
fi
git checkout -q --detach "$main"

# 5. Statuses.
echo >> "$report"
echo "Result: **$result**${step:+ (first failure: $step)}" >> "$report"
log "result: $result${step:+ ($step)}"
set_status() {
    local sha=$1 state=$2 desc=$3
    desc="${desc:0:139}"
    gh api --silent -X POST "repos/$REPO/statuses/$sha" -f state="$state" -f context="$CONTEXT" \
        -f description="$desc"
}
if [ "$status" = 1 ]; then
    for n in "${merged[@]}"; do
        if [ "$result" = pass ]; then
            set_status "${head[$n]}" success "Built and booted with main ${main:0:8} and $(echo "${merged[@]}" | wc -w) merge-next PRs; pi-test --starts $starts passed" || log "no status for #$n"
        else
            set_status "${head[$n]}" failure "Merge set failed at: $step. Run merge-set-check.sh --prs $n to isolate" || log "no status for #$n"
        fi
    done
    for n in "${conflicted[@]}"; do
        set_status "${head[$n]}" failure "Conflicts with main plus the merge-next PRs before it" || log "no status for #$n"
    done
    log "statuses set on ${#merged[@]} + ${#conflicted[@]} heads"
else
    log "--no-status: report only"
fi
echo "$key" > "$OUT/last-key"
log "report: $report"
[ "$result" = pass ]
