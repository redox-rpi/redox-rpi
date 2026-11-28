#!/usr/bin/env bash
# QEMU test for serverfs (#713): a read-only WebDAV directory as a scheme.
#
# Starts a throwaway WebDAV server (tools/serverfs/test/davserver.py) on
# 127.0.0.1 with a small test share under build/serverfs-test/, boots the
# image in QEMU (user network: the guest reaches the host's loopback as
# 10.0.2.2) and checks listing, reading, errors and the login, over plain
# HTTP and over HTTPS. For HTTPS it makes a throwaway CA and a server
# certificate for 10.0.2.2 (valid from 1970, as QEMU's guest has no RTC);
# the guest gets the CA as a PEM file. Nothing is installed on the host,
# and the servers end with the script.
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
tport=$((port + 3))
# Throwaway CA and server certificate for the HTTPS server.
tls="$work/tls"
mkdir -p "$tls"
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -subj /CN=serverfs-test-ca \
    -not_before 19700101000000Z -not_after 20991231000000Z \
    -keyout "$tls/ca.key" -out "$tls/ca.pem" 2> /dev/null
openssl req -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -subj /CN=10.0.2.2 \
    -keyout "$tls/server.key" -out "$tls/server.csr" 2> /dev/null
openssl x509 -req -in "$tls/server.csr" -CA "$tls/ca.pem" -CAkey "$tls/ca.key" \
    -not_before 19700101000000Z -not_after 20991231000000Z \
    -extfile <(printf 'subjectAltName=IP:10.0.2.2\nbasicConstraints=CA:FALSE\n') \
    -out "$tls/server.pem" 2> /dev/null
# The CA as one printf for the guest's shell (PEM lines are safe to type).
ca_cmd="printf '%s\\n' $(sed "s/.*/'&'/" "$tls/ca.pem" | tr '\n' ' ')> /tmp/ca.pem"

python3 "$ROOT/tools/serverfs/test/davserver.py" "$share" "$port" --auth sfuser:sfpass 2> "$work/dav.log" &
dav=$!
python3 "$ROOT/tools/serverfs/test/davserver.py" "$share" "$tport" --auth sfuser:sfpass \
    --tls "$tls/server.pem" "$tls/server.key" 2> "$work/davs.log" &
davs=$!
trap 'kill $dav $davs 2>/dev/null || true' EXIT
sleep 1
kill -0 "$dav" || { echo "the WebDAV server did not start, see $work/dav.log" >&2; exit 1; }
kill -0 "$davs" || { echo "the HTTPS WebDAV server did not start, see $work/davs.log" >&2; exit 1; }
url="http://10.0.2.2:$port/"
turl="https://10.0.2.2:$tport/"

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
    --run "date -u" \
    --run "$ca_cmd" \
    --run "serverfs -d --auth /tmp/sfauth --ca /tmp/ca.pem $turl tls; echo TLS-START \$?" \
    --run "echo TLS-COUNT \$(ls /scheme/tls | wc -l)" \
    --run "echo TLS-CAT \$(cat /scheme/tls/sub/nested.txt)" \
    --run "sha256sum /scheme/tls/big.bin" \
    --run "serverfs -d --auth /tmp/sfauth $turl tls2; ls /scheme/tls2; echo RC-NOCA \$?" \
    --run "serverfs -d --ca /tmp/ca.pem https://10.0.2.2:$port/ tls3; ls /scheme/tls3; echo RC-NOTLS \$?" \
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
check "https: starts"            "TLS-START 0"
check "https: lists 5 entries"   "TLS-COUNT 5"
check "https: reads a file"      "TLS-CAT nested file"
check "https: 3 MiB file intact" "$want +/scheme/tls/big.bin"
check "https: unknown CA refused" "RC-NOCA [1-9][0-9]*"
check "https: hint for the CA"   ".*UnknownIssuer.*--ca.*"
check "https to a plain port fails" "RC-NOTLS [1-9][0-9]*"
grep -aE "serverfs:|No such file|Read-only|ermission denied|route to host|nreachable|UTC" <<< "$log" | grep -av "^root:" | head -40
echo "requests: $(grep -c '"PROPFIND' "$work/dav.log") PROPFIND, $(grep -c '"GET' "$work/dav.log") GET (server log $work/dav.log)"
echo "https requests: $(grep -c '"PROPFIND' "$work/davs.log") PROPFIND, $(grep -c '"GET' "$work/davs.log") GET (server log $work/davs.log)"
[ "$fail" = 0 ] && echo "serverfs test: pass" || { echo "serverfs test: FAIL, see images/qemu-test.log"; exit 1; }
