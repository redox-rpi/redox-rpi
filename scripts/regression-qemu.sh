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
args=(127.0.0.1 --port "$DEVSH_PORT" --serial-log "$console" --tcp-host 10.0.2.2 --dead-dns 10.0.2.99
      --report "$report")
[ -n "$manifest" ] && args+=(--manifest "$manifest")
rc=0
python3 scripts/regression-set.py "${args[@]}" "${pass[@]}" || rc=$?
log "regression set: $([ "$rc" = 0 ] && echo pass || echo FAIL) in $((SECONDS - start)) s with the boot; report $report"
exit "$rc"
