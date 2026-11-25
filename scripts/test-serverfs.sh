#!/usr/bin/env bash
# QEMU test for serverfs (#713): a read-only WebDAV directory as a scheme.
#
# Starts a throwaway WebDAV server (tools/serverfs/test/davserver.py) on
# 127.0.0.1 with a small test share under build/serverfs-test/, boots the
# image in QEMU (user network: the guest reaches the host's loopback as
# 10.0.2.2) and checks listing, reading, errors and the login. Nothing is
# installed on the host, and the server ends with the script.
#
# Needs an image built with --experiment serverfs.
#
# Usage: scripts/test-serverfs.sh [--timeout <seconds>]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
timeout=600
[ "${1:-}" = "--timeout" ] && timeout="$2"

work="$ROOT/build/serverfs-test"
share="$work/share"
rm -rf "$work"
mkdir -p "$share/sub" "$share/empty"
printf 'hello from the server\n' > "$share/hello.txt"
printf 'nested file\n' > "$share/sub/nested.txt"
printf 'spaces and umlaut\n' > "$share/a b ä.txt"
head -c 3145728 /dev/urandom > "$share/big.bin"
want="$(sha256sum < "$share/big.bin" | cut -d' ' -f1)"

port="${SERVERFS_TEST_PORT:-$((20000 + RANDOM % 20000))}"
python3 "$ROOT/tools/serverfs/test/davserver.py" "$share" "$port" --auth sfuser:sfpass 2> "$work/dav.log" &
dav=$!
trap 'kill $dav 2>/dev/null || true' EXIT
sleep 1
kill -0 "$dav" || { echo "the WebDAV server did not start, see $work/dav.log" >&2; exit 1; }
url="http://10.0.2.2:$port/"

rc=0
# User networking with its host port forwards on free ports (parallel runs).
export DEVSH_PORT=$((port + 1)) SSH_PORT=$((port + 2))
scripts/qemu-raspi3b.sh --usb-net --test --timeout "$timeout" \
    --run "sleep 15" \
    --run "echo sfuser:sfpass > /tmp/sfauth" \
    --run "serverfs -d --auth /tmp/sfauth $url server; echo SF-START \$?" \
    --run "ls -l /scheme/server" \
    --run "echo LS-COUNT \$(ls /scheme/server | wc -l)" \
    --run "cat /scheme/server/hello.txt" \
    --run "cat /scheme/server/sub/nested.txt" \
    --run "cat '/scheme/server/a b ä.txt'" \
    --run "echo LS-EMPTY \$(ls /scheme/server/empty | wc -l)" \
    --run "sha256sum /scheme/server/big.bin" \
    --run "cat /scheme/server/missing.txt; echo RC-MISSING \$?" \
    --run "touch /scheme/server/new.txt; echo RC-WRITE \$?" \
    --run "echo sfuser:wrong > /tmp/sfbad" \
    --run "serverfs -d --auth /tmp/sfbad $url server2; ls /scheme/server2; echo RC-BADAUTH \$?" \
    --run "serverfs -d http://10.0.2.2:1/ server3; ls /scheme/server3; echo RC-DOWN \$?" \
    > /dev/null 2>&1 || rc=$?
log="$(tr -d '\r' < images/qemu-test.log | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g;s/\x1b\][^\x07]*\x07//g')"
[ "$rc" = 0 ] || { echo "QEMU run failed (rc $rc), see images/qemu-test.log" >&2; exit 1; }

fail=0
check() { # <name> <extended regex on a whole line>
    if grep -aqE "^$2\$" <<< "$log"; then echo "pass  $1"; else echo "FAIL  $1 (no line matching $2)"; fail=1; fi
}
check "starts and registers"     "SF-START 0"
check "lists 5 entries"          "LS-COUNT 5"
check "reads a file"             "hello from the server"
check "reads in a subdirectory"  "nested file"
check "name with space, umlaut"  "spaces and umlaut"
check "empty directory"          "LS-EMPTY 0"
check "3 MiB file intact"        "$want +/scheme/server/big.bin"
check "missing file fails"       "RC-MISSING [1-9][0-9]*"
check "write is refused"         "RC-WRITE [1-9][0-9]*"
check "wrong password refused"   "RC-BADAUTH [1-9][0-9]*"
check "server down fails"        "RC-DOWN [1-9][0-9]*"
grep -aE "serverfs:|No such file|Read-only|ermission denied|route to host|nreachable" <<< "$log" | grep -av "^root:" | head -20
echo "requests: $(grep -c '"PROPFIND' "$work/dav.log") PROPFIND, $(grep -c '"GET' "$work/dav.log") GET (server log $work/dav.log)"
[ "$fail" = 0 ] && echo "serverfs test: pass" || { echo "serverfs test: FAIL, see images/qemu-test.log"; exit 1; }
