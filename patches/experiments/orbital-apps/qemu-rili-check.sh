#!/usr/bin/env bash
# QEMU check for Ri-li (#528): starts the desktop, then Ri_li; prints its
# log, whether it runs and the audio counter before and after, and takes two
# screenshots (images/rili.ppm, images/rili.ppm.2: the loading screen and
# the language menu). Build with the desktop experiments (see
# ../desktop/README.md). Usage: qemu-rili-check.sh [image]
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
image="${1:-$ROOT/upstream/redox/build/aarch64/minimal/harddrive.img}"
mon=$ROOT/images/qemu-monitor-rili.sock
rm -f "$mon"
H=/scheme/audiohw
( for _ in $(seq 400); do [ "$(tr -d '\r' < "$ROOT/images/qemu-test.log" 2>/dev/null | grep -ac "^loud_chunks")" -ge 2 ] && break; sleep 1; done; python3 - "$mon" "$ROOT/images/rili.ppm" <<'PY'
import socket, sys, time
for _ in range(60):
    try:
        m = socket.socket(socket.AF_UNIX); m.connect(sys.argv[1]); break
    except OSError:
        time.sleep(2)
m.sendall(f"screendump {sys.argv[2]}\n".encode()); time.sleep(50)
m.sendall(f"screendump {sys.argv[2]}.2\n".encode()); time.sleep(3)
PY
) &
"$ROOT/scripts/qemu-raspi3b.sh" --image "$image" --timeout 900 --monitor "$mon" \
    --run "desktop --yes" \
    --run "grep loud_chunks $H/stats" \
    --run "Ri_li &> /tmp/rili.log &" \
    --run "sleep 25" \
    --run "cat /tmp/rili.log" \
    --run "ps | grep -i ri_li" \
    --run "grep loud_chunks $H/stats" \
    --run "sleep 90" > /dev/null 2>&1
echo "qemu status $?"
tr -d '\r' < "$ROOT/images/qemu-test.log" | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g' | grep -aiE -A3 "rili|ri_li|loud_chunks|orbital window|audio driver|Impossible|error" | grep -v "^--" | tail -40
ls -la "$ROOT/images/rili.ppm"
