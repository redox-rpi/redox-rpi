#!/usr/bin/env bash
# Boot the network boot image in QEMU raspi3b, run commands as root through
# devshd, print their output without terminal escape sequences, stop QEMU.
# The serial console (kernel log, crash dumps) ends up in images/qrun.log.
#
# Usage:
#   scripts/qemu-run.sh [--image IMG] 'command' ['command']...
#
# Example (ion syntax), the reproducer for the random program failures:
#   scripts/qemu-run.sh 'for i in 1..=15; sha256sum /etc/issue > /tmp/o$i; echo $i rc=$?; end'
#   grep -E 'ELR_EL1|ESR_EL1|NAME' images/qrun.log

set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
image=upstream/redox/build/aarch64/minimal/harddrive.img
if [ "${1:-}" = --image ]; then
    image=$2
    shift 2
fi

# The bracket keeps pkill from matching this script's own command line.
pkill -f "[q]emu-system-aarch64.*raspi3b" 2>/dev/null
sleep 1
timeout 900 scripts/qemu-raspi3b.sh --usb-net --image "$image" \
    < /dev/null > images/qrun.log 2>&1 &

# devshd starts late, after the console's login prompt.
for _ in $(seq 80); do
    python3 scripts/devsh.py 127.0.0.1 12323 "echo ready" 2>/dev/null | grep -q ready && break
    sleep 3
done

python3 scripts/devsh.py 127.0.0.1 12323 "$@" 2>&1 | python3 -c '
import re, sys
s = sys.stdin.read()
s = re.sub(r"\x1b\[[0-9;?]*[a-zA-Z]|\x1b\][^\x07]*\x07", "", s).replace("\x00", "")
for line in s.replace("\r", "\n").split("\n"):
    # Skip the prompt redraws Ion sends for every typed character.
    if line.strip() and "root:~#" not in line and "⏎" not in line \
            and "login:" not in line and not line.startswith("#"):
        print(line[:400])
'

pkill -f "[q]emu-system-aarch64.*raspi3b" 2>/dev/null
true
