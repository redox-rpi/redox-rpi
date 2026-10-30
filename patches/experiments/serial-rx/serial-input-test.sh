#!/usr/bin/env bash
# Send lines of 200 characters at full speed to the Pi's serial console and
# check that the shell runs each one complete (#28). Needs a root shell on
# the console (log in first) and the Pi from the test queue.
#
# Usage (on the build host):
#   patches/experiments/serial-rx/serial-input-test.sh [count]
#
# Reads the console from the logger's file, never from the device.

set -euo pipefail

COUNT="${1:-20}"
DEV=/dev/ttyUSB0
LOG="$HOME/redox-rpi/logs/serial.log"
# 200 characters: 000A001B002C...049X
LINE="$(python3 -c 'print("".join(f"{i:03d}" + chr(65 + i % 26) for i in range(50)))')"

pass=0
for ((n = 1; n <= COUNT; n++)); do
    tag="$(printf 'S%02d' "$n")"
    start=$(stat -c %s "$LOG")
    # One write: the whole command at full speed.
    printf 'echo %s%s\r' "$tag" "$LINE" > "$DEV"
    # Wait for the output line: the tag and the 200 characters, alone on a line.
    ok=no
    for _ in $(seq 1 30); do
        if tail -c +$((start + 1)) "$LOG" | tr -d '\r\000' | grep -aqx "$tag$LINE"; then
            ok=yes
            break
        fi
        sleep 1
    done
    if [ "$ok" = yes ]; then
        pass=$((pass + 1))
        echo "$tag complete"
    else
        echo "$tag LOST characters"
        # Clear what is left of the line before the next one.
        printf '\025\r' > "$DEV"
        sleep 2
    fi
done
echo "$pass of $COUNT lines complete"
[ "$pass" -eq "$COUNT" ]
