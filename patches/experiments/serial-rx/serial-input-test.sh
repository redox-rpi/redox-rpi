#!/usr/bin/env bash
# Send lines of 200 characters at full speed to the Pi's serial console and
# check that the shell runs each one complete (#28). Needs a root shell on
# the console (log in first) and the Pi from the test queue.
#
# Usage (on the build host):
#   patches/experiments/serial-rx/serial-input-test.sh [count] [wait]
#
#   count  lines to send (default 20)
#   wait   seconds to wait for each line's output (default 120)
#
# Reads the console from the logger's file, never from the device.
#
# The shell redraws the whole line for each character it reads, about 31 KB
# of console output for each 209-byte command, so one line takes several
# seconds at 115200 baud (Test 490). A line whose output does not show up
# within the wait is not cleared; it is checked again at the end over the
# whole log of the run:
#   complete  output seen within the wait
#   late      output only seen in the final check (slow, nothing lost)
#   LOST      output never seen complete (characters lost)
# The script passes when no line is LOST.

set -euo pipefail

COUNT="${1:-20}"
WAIT="${2:-120}"
DEV=/dev/ttyUSB0
LOG="$HOME/redox-rpi/logs/serial.log"
# 200 characters: 000A001B002C...049X
LINE="$(python3 -c 'print("".join(f"{i:03d}" + chr(65 + i % 26) for i in range(50)))')"

has_line() { # has_line <from-offset> <tag>
    tail -c +$(($1 + 1)) "$LOG" | tr -d '\r\000' | grep -aqx "$2$LINE"
}

run_start=$(stat -c %s "$LOG")
complete=0
pending=()
for ((n = 1; n <= COUNT; n++)); do
    tag="$(printf 'S%02d' "$n")"
    start=$(stat -c %s "$LOG")
    t0=$(date +%s)
    # One write: the whole command at full speed.
    printf 'echo %s%s\r' "$tag" "$LINE" > "$DEV"
    # Wait for the output line: the tag and the 200 characters, alone on a line.
    ok=no
    while [ $(($(date +%s) - t0)) -lt "$WAIT" ]; do
        if has_line "$start" "$tag"; then
            ok=yes
            break
        fi
        sleep 1
    done
    if [ "$ok" = yes ]; then
        complete=$((complete + 1))
        echo "$tag complete $(($(date +%s) - t0)) s"
    else
        echo "$tag no output after $WAIT s, checked again at the end"
        pending+=("$tag")
    fi
done

# Final check over the whole run: give the console time to drain first.
late=0
lost=()
if [ ${#pending[@]} -gt 0 ]; then
    sleep "$WAIT"
    for tag in "${pending[@]}"; do
        if has_line "$run_start" "$tag"; then
            late=$((late + 1))
            echo "$tag late (complete in the final check)"
        else
            lost+=("$tag")
            echo "$tag LOST characters"
        fi
    done
fi
echo "$complete of $COUNT lines complete, $late late, ${#lost[@]} lost${lost[*]:+ (${lost[*]})}"
[ ${#lost[@]} -eq 0 ]
