#!/usr/bin/env bash
# Print the test queue: queue.md on main of redox-rpi/pi-tests, where the
# queue and the reports live since RFC 0002 phase 2.
#
#   scripts/pi-tests-queue.sh > queue.md
#
# Uses the clone in $PI_TESTS_CLONE (default ~/pi-tests-stable, the
# nightly's) after a fetch, so a row marked "on the Pi" a minute ago counts.
# Without a clone it downloads the file, which GitHub may cache for a few
# minutes. Exits non-zero if neither works.

set -euo pipefail
clone="${PI_TESTS_CLONE:-$HOME/pi-tests-stable}"
if [ -d "$clone/.git" ]; then
    git -C "$clone" fetch -q origin 2>/dev/null || true
    git -C "$clone" show origin/main:queue.md && exit 0
fi
curl -fsS -m 30 https://raw.githubusercontent.com/redox-rpi/pi-tests/main/queue.md
