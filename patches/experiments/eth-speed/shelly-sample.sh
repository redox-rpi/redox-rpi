#!/usr/bin/env bash
# Average the Pi's power from the Shelly plug (#126). Read only: it never
# switches the plug.
#
# Usage: shelly-sample.sh <label> [samples] [seconds between]
# Prints each reading on one line, then: <label>: mean X.XX W, min, max, n.

set -uo pipefail
label="${1:?label}"
n="${2:-18}"
gap="${3:-10}"
# The plug's status URL (.../rpc/Switch.GetStatus?id=0): REDOX_SHELLY_URL
# from the environment or lab.env (scripts/lab-env.sh).
. "$(cd "$(dirname "$0")/../../.." && pwd)/scripts/lab-env.sh"
lab_need REDOX_SHELLY_URL
url="$REDOX_SHELLY_URL"
values=()
for ((i = 0; i < n; i++)); do
    v="$(curl -s -m 15 "$url" | grep -oE '"apower":[0-9.]+' | cut -d: -f2)"
    [ -n "$v" ] && values+=("$v")
    [ $i -lt $((n - 1)) ] && sleep "$gap"
done
echo "${values[*]}"
printf '%s\n' "${values[@]}" | awk -v label="$label" '
    { s += $1; if (NR == 1 || $1 < lo) lo = $1; if (NR == 1 || $1 > hi) hi = $1 }
    END { if (NR) printf "%s: mean %.2f W, min %.1f, max %.1f, n %d\n", label, s / NR, lo, hi, NR; else print label ": no readings" }'
