#!/usr/bin/env python3
"""Compare a relibc test suite run with the list of known failures (#13).

Usage: scripts/relibc-tests-compare.py <serial log> [<expected list>] [<build ID>]

The serial log is the console output of `ion /usr/share/relibc-tests/run.ion`
(scripts/relibc-tests.sh). The expected list has one test per line:
`<test> fail` (known to fail) or `<test> flaky` (passes or fails), each
optionally followed by `# comment`.

Prints NEW and FIXED lines and a SUMMARY. Exit status: 0 no new failure,
1 new failure, 2 the suite did not finish.
"""

import re
import sys

ANSI = re.compile(r"\x1b\[[0-9;?]*[a-zA-Z]|\x1b\][^\x07]*\x07")
TEST = re.compile(r"^# (\./bins_\S+) #$")
# "<test> failed - ...", "# <test>: process exited ...", "<test>: assumed hangs ..."
FAILURE = re.compile(r"^(?:# )?(\./bins_\S+?)(?::| failed)")
DONE = re.compile(r"RELIBC-TESTS rc=(\d+)")


def read_expected(path):
    expected = {}
    with open(path) as f:
        for number, line in enumerate(f, 1):
            line = line.split("#", 1)[0].strip()
            if not line:
                continue
            words = line.split()
            if len(words) != 2 or words[1] not in ("fail", "flaky"):
                sys.exit(f"{path}:{number}: expected '<test> fail' or '<test> flaky'")
            expected[words[0]] = words[1]
    return expected


def main():
    if not 2 <= len(sys.argv) <= 4:
        sys.exit(__doc__.strip().splitlines()[2])
    log = sys.argv[1]
    expected = read_expected(sys.argv[2] if len(sys.argv) > 2 else "config/relibc-tests-expected.txt")
    build_id = sys.argv[3] if len(sys.argv) > 3 else log

    with open(log, errors="replace") as f:
        lines = [ANSI.sub("", line).replace("\r", "").rstrip() for line in f]

    ran = [m.group(1) for m in map(TEST.match, lines) if m]
    done = [i for i, line in enumerate(lines) if DONE.search(line)]
    if not ran or not done:
        print(f"ERROR {build_id}: the suite did not finish "
              f"({len(ran)} tests started, no 'RELIBC-TESTS rc=' line)")
        return 2

    # Failures: the lines between the last "# FAILURES #" and the rc line.
    end = done[-1]
    start = max((i for i, line in enumerate(lines[:end]) if line == "# FAILURES #"), default=end)
    failed = {}
    for line in lines[start + 1:end]:
        m = FAILURE.match(line)
        if m:
            failed.setdefault(m.group(1), line)

    new = sorted(t for t in failed if t not in expected)
    fixed = sorted(t for t, kind in expected.items() if kind == "fail" and t in ran and t not in failed)
    flaky = sorted(t for t in failed if expected.get(t) == "flaky")
    missing = sorted(t for t in expected if t not in ran)

    for t in new:
        print(f"NEW {t}: {failed[t]}")
    for t in fixed:
        print(f"FIXED {t}: on the list, but passed")
    for t in missing:
        print(f"NOT RUN {t}: on the list, but not in this run")
    print(f"SUMMARY {build_id}: {len(ran)} tests, {len(failed)} failed "
          f"({len(failed) - len(new) - len(flaky)} known, {len(flaky)} flaky, {len(new)} new), "
          f"{len(fixed)} fixed")
    return 1 if new else 0


if __name__ == "__main__":
    sys.exit(main())
