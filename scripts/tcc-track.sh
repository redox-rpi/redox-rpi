#!/usr/bin/env bash
# Nightly TinyCC VC4 track, host part (#410, WBS 2.15 point 3): build the
# rpi-open-firmware bootcode (MONITOR=1) with vc4-elf-gcc (G) and with
# vc4-tcc (T, linked by vc4-tcc) from the pins in config/tcc-track.lock,
# and run the resim tests on them. Not gating: scripts/update-stable.sh runs
# it after the Redox part and promotes the stable image whatever it says.
#
#   scripts/tcc-track.sh --out <dir> [--work <dir>]
#
#   --out   writes G.bin, T.bin, host.json and the logs there
#   --work  clones of redox-rpi/tinycc and rpi-open-firmware and the resim
#           build (default ~/tcc-track); only this script uses them
#
# host.json: {"tinycc", "fat_boot", "gcc", "resim_pass", "size_g", "size_t",
# "sha256_g", "sha256_t", "checks": {name: "pass"|"fail"}}. resim_pass is
# true when abi-sim and fat-sim pass (fat-sim runs TCC's and GCC's fat.c;
# GCC's run is informational there). The Pi part (scripts/tcc-track-pi.py)
# loads G.bin and T.bin over serial;
# scripts/tcc-track-record.py writes the night's record from both.
#
# Exit 0 when host.json was written (even with failed checks), 1 when the
# setup failed (toolchain, clones); then there is no host.json.
#
# vc4-elf toolchain: $VC4_PREFIX (default
# ~/rbc-build/toolchains/prefix/bin/vc4-elf-), from build-toolchain.sh.

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
. "$ROOT/config/tcc-track.lock"
out="" work="$HOME/tcc-track"
while [ $# -gt 0 ]; do
    case "$1" in
        --out) out="$2"; shift 2 ;;
        --work) work="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done
[ -n "$out" ] || { echo "usage: $0 --out <dir> [--work <dir>]" >&2; exit 1; }
mkdir -p "$out" "$work"
out="$(cd "$out" && pwd)" work="$(cd "$work" && pwd)"
P="${VC4_PREFIX:-$HOME/rbc-build/toolchains/prefix/bin/vc4-elf-}"
log() { echo "$(date -u +%H:%M:%S) tcc-track: $*"; }

gcc_version="$("${P}gcc" -dumpversion 2> /dev/null || true)"
[ "$gcc_version" = "$VC4_GCC" ] ||
    { log "${P}gcc is '${gcc_version:-missing}', the pin says $VC4_GCC"; exit 1; }

clone() { # repo dir rev
    [ -d "$2/.git" ] || git clone -q "https://github.com/$1.git" "$2"
    git -C "$2" fetch -q origin
    git -C "$2" checkout -q --detach "$3"
    git -C "$2" clean -qfdx
}
tcc="$work/tinycc" fw="$work/rpi-open-firmware"
clone redox-rpi/tinycc "$tcc" "$TINYCC_REV" || { log "no tinycc at $TINYCC_REV"; exit 1; }
clone redox-rpi/rpi-open-firmware "$fw" "$FAT_BOOT_REV" || { log "no fat-boot at $FAT_BOOT_REV"; exit 1; }
tinycc_full="$(git -C "$tcc" rev-parse HEAD)" fat_full="$(git -C "$fw" rev-parse HEAD)"
log "tinycc ${tinycc_full:0:10}, fat-boot ${fat_full:0:10}, vc4-elf-gcc $gcc_version"

(cd "$tcc" && ./configure --config-bcheck=no --config-backtrace=no && make vc4-tcc) \
    > "$out/vc4-tcc.log" 2>&1 || { log "vc4-tcc does not build, see $out/vc4-tcc.log"; exit 1; }
E="$("$tcc/tests/vc4/ci/build-resim.sh" "$work/resim" 2> "$out/resim.log")" ||
    { log "resim does not build, see $out/resim.log"; exit 1; }

declare -A checks
run() { # name command...
    local name=$1; shift
    if "$@" > "$out/$name.log" 2>&1; then checks[$name]=pass; else checks[$name]=fail; fi
    log "$name: ${checks[$name]}"
}
rm -rf "$out/keep"
run bootcode env LINK=tcc KEEP="$out/keep" "$tcc/tests/vc4/bootcode-build.sh" "$fw" "$P"
cp "$fw/bootcode/build/bootcode.bin" "$out/G.bin" 2> /dev/null || rm -f "$out/G.bin"
cp "$out/keep/bootcode-tcc.bin" "$out/T.bin" 2> /dev/null || rm -f "$out/T.bin"
run abi-sim "$tcc/tests/vc4/abi-sim.sh" "$P" "$E"
run fat-sim "$tcc/tests/vc4/fat-sim.sh" "$fw" "$P" "$E"

checks_json="$(for k in "${!checks[@]}"; do echo "$k=${checks[$k]}"; done)"
python3 - "$out" "$tinycc_full" "$fat_full" "$gcc_version" "$VC4_TOOLCHAIN_REV" <<PY
import hashlib, json, os, sys
out, tinycc, fat, gcc, tc = sys.argv[1:]
checks = dict(l.split("=", 1) for l in """$checks_json""".split())
def info(name):
    p = os.path.join(out, name)
    if not os.path.exists(p):
        return None, None
    return os.path.getsize(p), hashlib.sha256(open(p, "rb").read()).hexdigest()
size_g, sha_g = info("G.bin")
size_t, sha_t = info("T.bin")
rec = {"tinycc": tinycc[:12], "fat_boot": fat[:12], "gcc": f"{gcc} vc4-toolchain {tc}",
       "resim_pass": checks.get("abi-sim") == "pass" and checks.get("fat-sim") == "pass"
                     and checks.get("bootcode") == "pass",
       "size_g": size_g, "size_t": size_t, "sha256_g": sha_g, "sha256_t": sha_t,
       "checks": checks}
json.dump(rec, open(os.path.join(out, "host.json"), "w"), indent=1)
print(json.dumps(rec))
PY
