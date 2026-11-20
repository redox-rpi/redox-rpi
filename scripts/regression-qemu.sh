#!/usr/bin/env bash
# Fast regression set before a Pi row (#265): boot a built network-boot image
# in QEMU (raspi3b, USB network, data partition) and run
# scripts/regression-set.py against it. Under 10 minutes on the build host;
# meant for every PR that changes patches/, config/ or recipes, before its
# Pi row is queued.
#
#   scripts/regression-qemu.sh [--image <harddrive.img>] [--manifest <file>]
#                              [--out <dir>] [regression-set.py options]...
#
#   --image     the image to boot (default: the last network-boot build of
#               this clone, upstream/redox/build/aarch64/minimal/harddrive.img);
#               it is copied, never changed
#   --manifest  its manifest (default: build/manifests/netboot-latest.toml
#               when --image is the default)
#   --out       where the console log and the report go (default:
#               build/regression/<UTC time>)
#   Every other option goes to scripts/regression-set.py, e.g.
#   --only dns --only tcp, or --starts 100.
#
# The dns-outside check looks up a name outside the LAN
# (REGRESSION_OUTSIDE_NAME, default static.redox-os.org) through QEMU's user
# network, so it needs internet access on this host; set
# REGRESSION_OUTSIDE_NAME= (empty) to skip it offline, or pass
# --outside-name <name> for another name.
#
# After the regression-set boot, checks that need a QEMU boot of their own run one after
# the other, each skipped while its script is not in the clone:
#   tcp-reset   #141: scripts/test-tcp-reset-read.sh (socket netdev, PR #454)
#
# One QEMU per clone (images/run.img is shared): run nothing else in QEMU in
# this clone meanwhile. DEVSH_PORT and SSH_PORT pick the host ports
# (default 12721, 12722) for parallel runs in different clones.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
DEVSH_PORT="${DEVSH_PORT:-12721}"
SSH_PORT="${SSH_PORT:-12722}"
LOGIN_TIMEOUT=600
image=""
manifest=""
out=""
pass=()
while [ $# -gt 0 ]; do
    case "$1" in
        --image) image="$2"; shift 2 ;;
        --manifest) manifest="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        *) pass+=("$1"); shift ;;
    esac
done
if [ -z "$image" ]; then
    image=upstream/redox/build/aarch64/minimal/harddrive.img
    [ -n "$manifest" ] || manifest="$(readlink -f build/manifests/netboot-latest.toml 2> /dev/null || true)"
fi
[ -f "$image" ] || { echo "no image: $image" >&2; exit 2; }
out="${out:-build/regression/$(date -u +%Y%m%dT%H%M%S)}"
mkdir -p "$out"
log() { echo "$(date -u +%H:%M:%S) $*"; }

copy="$out/harddrive.img"
cp "$image" "$copy"
console="$out/console.log"
report="$out/report.md"
: > "$console"
start=$SECONDS
log "booting $image in QEMU (devsh on port $DEVSH_PORT)"
DEVSH_PORT="$DEVSH_PORT" SSH_PORT="$SSH_PORT" timeout 1800 \
    scripts/qemu-raspi3b.sh --usb-net --data --image "$copy" > "$console" 2>&1 &
qemu=$!
stop_qemu() {
    pkill -f "hostfwd=tcp::$DEVSH_PORT-" 2> /dev/null || true
    wait "$qemu" 2> /dev/null || true
    rm -f "$copy"
}
trap stop_qemu EXIT
until tr -d '\r' < "$console" | grep -aq 'redox login:'; do
    sleep 2
    if [ $((SECONDS - start)) -ge "$LOGIN_TIMEOUT" ] || ! kill -0 "$qemu" 2> /dev/null; then
        log "FAIL boot: no login prompt within $((SECONDS - start)) s (log: $console)"
        echo "## Regression set: FAIL boot, no login prompt within $((SECONDS - start)) s" >> "$report"
        exit 1
    fi
done
log "login prompt after $((SECONDS - start)) s"
# devshd and DHCP need a moment after the prompt.
for _ in $(seq 60); do
    python3 scripts/devsh.py 127.0.0.1 "$DEVSH_PORT" true > /dev/null 2>&1 && break
    sleep 2
done
# QEMU's USB network moves about 0.1 MiB/s, so 1 MiB is enough for the TCP check.
args=(127.0.0.1 --port "$DEVSH_PORT" --serial-log "$console" --tcp-host 10.0.2.2 --tcp-mib 1
      --dead-dns 10.0.2.99 --report "$report")
outside="${REGRESSION_OUTSIDE_NAME-static.redox-os.org}"
[ -n "$outside" ] && args+=(--outside-name "$outside")
[ -n "$manifest" ] && args+=(--manifest "$manifest")
rc=0
python3 scripts/regression-set.py "${args[@]}" "${pass[@]}" || rc=$?
stop_qemu

# Checks that need a QEMU boot of their own, after the first one has
# stopped (one QEMU per clone). Each is skipped while its script is not in
# this clone, so it starts with the PR that brings it. --only limits them
# like the regression-set checks.
only=()
for i in "${!pass[@]}"; do
    [ "${pass[$i]}" = --only ] && only+=("${pass[$((i + 1))]:-}")
done
own_header=""
own_boot() { # name, guards, script, script args...
    local name="$1" guards="$2" script="$3" word detail r=0
    shift 3
    if [ ${#only[@]} -gt 0 ] && ! printf '%s\n' "${only[@]}" | grep -qx "$name"; then
        return 0
    fi
    if [ ! -x "$script" ]; then
        word=SKIP detail="no $script in this clone ($guards)"
    else
        log "$name: own QEMU boot ($script)"
        "$script" "$@" > "$out/$name.log" 2>&1 || r=$?
        cp images/qemu-test.log "$out/$name-console.log" 2> /dev/null || true
        detail="$(grep -aE '^(PASS|FAIL)' "$out/$name.log" | tail -1 || true)"
        detail="${detail#PASS: }"; detail="${detail#FAIL: }"
        detail="${detail:-exit $r, see $name.log}"
        if [ "$r" = 0 ]; then word=PASS; else word=FAIL; rc=1; fi
    fi
    echo "$word $name: $detail"
    if [ -z "$own_header" ]; then
        printf '## Own QEMU boots\n\n| Check | Result | Detail |\n|---|---|---|\n' >> "$report"
        own_header=yes
    fi
    echo "| $name | $word | ${detail//|//} |" >> "$report"
}
# #141: a read after a TCP reset returns the buffered data first (PR #454).
own_boot tcp-reset "#141, #454" scripts/test-tcp-reset-read.sh --image "$image"

log "regression set: $([ "$rc" = 0 ] && echo pass || echo FAIL) in $((SECONDS - start)) s with the boot; report $report"
exit "$rc"
