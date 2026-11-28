#!/usr/bin/env bash
# QEMU test for homesync (#712): back up the home directory with rsync over
# SSH.
#
# Starts a throwaway sshd as the calling user on 127.0.0.1 (a random high
# port, its own host key and authorized_keys under build/homesync-test/;
# the guest reaches it as 10.0.2.2), boots the image in QEMU and runs
# setup, push, a second push after one change, a push after one delete,
# pull, status and a push to a server that is not there. The server
# directory is checked on the host. Nothing is installed on the host, and
# sshd ends with the script.
#
# Needs an image built with --experiment sshd --experiment timesync
# --experiment homesync, and sshd and rsync on the host.
#
# Usage: scripts/test-homesync.sh [--timeout <seconds>]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
timeout=900
[ "${1:-}" = "--timeout" ] && timeout="$2"

work="$ROOT/build/homesync-test"
rm -rf "$work"
mkdir -p "$work/server" "$work/ssh"
chmod 700 "$work/ssh"
ssh-keygen -q -t ed25519 -N '' -f "$work/ssh/host_key"
ssh-keygen -q -t ed25519 -N '' -C homesync-test -f "$work/ssh/client_key"
cp "$work/ssh/client_key.pub" "$work/ssh/authorized_keys"

port="${HOMESYNC_TEST_PORT:-$((20000 + RANDOM % 20000))}"
cat > "$work/ssh/sshd_config" <<EOF
Port $port
ListenAddress 127.0.0.1
HostKey $work/ssh/host_key
AuthorizedKeysFile $work/ssh/authorized_keys
PidFile $work/ssh/sshd.pid
PasswordAuthentication no
KbdInteractiveAuthentication no
UsePAM no
StrictModes no
AllowUsers $(id -un)
EOF
/usr/sbin/sshd -D -e -f "$work/ssh/sshd_config" 2> "$work/sshd.log" &
sshd=$!
trap 'kill $sshd 2>/dev/null || true' EXIT
sleep 1
kill -0 "$sshd" || { echo "sshd did not start, see $work/sshd.log" >&2; exit 1; }

server="$(id -un)@10.0.2.2:$port:$work/server"
# ion reads @name as an array, so the user@host specs are in single quotes.
# The client key as printf lines for the guest's shell (the key is a
# throwaway made above; base64 lines are safe to type).
key_cmd="printf '%s\\n' $(sed "s/.*/'&'/" "$work/ssh/client_key" | tr '\n' ' ')> /home/root/.ssh/id_ed25519"
pub_cmd="printf '%s\\n' '$(cat "$work/ssh/client_key.pub")' > /home/root/.ssh/id_ed25519.pub"

rc=0
export DEVSH_PORT=$((port + 1)) SSH_PORT=$((port + 2))
scripts/qemu-raspi3b.sh --usb-net --test --timeout "$timeout" \
    --run "sleep 15" \
    --run "mkdir -p /home/root/.ssh /home/root/data /home/root/.cache/junk" \
    --run "$key_cmd" \
    --run "$pub_cmd" \
    --run "chmod 600 /home/root/.ssh/id_ed25519" \
    --run "for i in 1..=100; head -c 65536 /dev/urandom > /home/root/data/f\$i; end" \
    --run "echo cache > /home/root/.cache/junk/x" \
    --run "echo DATA-COUNT \$(ls /home/root/data | wc -l)" \
    --run "homesync setup '$server'; echo RC-SETUP \$?" \
    --run "homesync push; echo RC-PUSH1 \$?" \
    --run "head -c 65536 /dev/urandom > /home/root/data/f7" \
    --run "homesync push; echo RC-PUSH2 \$?" \
    --run "rm /home/root/data/f9" \
    --run "homesync push; echo RC-PUSH3 \$?" \
    --run "homesync status; echo RC-STATUS \$?" \
    --run "homesync pull data; echo RC-PULL \$?" \
    --run "echo SUM-A \$(cat /home/root/data/* | sha256sum)" \
    --run "echo SUM-B \$(cat /home/root/restore-*/data/* | sha256sum)" \
    --run "mkdir -p /tmp/h2/.config/homesync" \
    --run "echo 'root@10.0.2.2:1:/nowhere' > /tmp/h2/.config/homesync/server" \
    --run "env HOME=/tmp/h2 homesync push; echo RC-DOWN \$?" \
    > /dev/null 2>&1 || rc=$?
log="$(tr -d '\r' < images/qemu-test.log | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g;s/\x1b\][^\x07]*\x07//g')"
[ "$rc" = 0 ] || { echo "QEMU run failed (rc $rc), see images/qemu-test.log" >&2; exit 1; }

fail=0
check() { # <name> <extended regex on a whole line>
    if grep -aqE "^$2\$" <<< "$log"; then echo "pass  $1"; else echo "FAIL  $1 (no line matching $2)"; fail=1; fi
}
host() { # <name> <condition>
    if eval "$2"; then echo "pass  $1"; else echo "FAIL  $1 ($2)"; fail=1; fi
}
check "100 test files"            "DATA-COUNT 100"
check "setup: login works"        "homesync: login to .* works.*"
check "setup exit 0"              "RC-SETUP 0"
check "first push"                "RC-PUSH1 0"
check "second push"               "RC-PUSH2 0"
check "third push"                "RC-PUSH3 0"
check "status: reachable"         "reachable: yes"
check "status exit 0"             "RC-STATUS 0"
check "pull exit 0"               "RC-PULL 0"
check "server away: message"      "homesync: server 10.0.2.2 port 1 not reachable, nothing changed"
check "server away: exit 1"       "RC-DOWN 1"
a="$(grep -aE '^SUM-A ' <<< "$log" | awk '{print $2}')"
b="$(grep -aE '^SUM-B ' <<< "$log" | awk '{print $2}')"
host "pull: same content"         '[ -n "$a" ] && [ "$a" = "$b" ]'
cur="$work/server/current"
host "server: 99 files now"       '[ "$(ls "$cur/data" 2>/dev/null | wc -l)" = 99 ]'
host "server: f9 gone"            '[ ! -e "$cur/data/f9" ]'
host "server: no .cache"          '[ ! -e "$cur/.cache" ]'
# deleted/<date>/data/ holds the old f7 (second push) and f9 (third push).
olds="$(cd "$work/server/deleted" 2>/dev/null && find . -path '*/data/*' -type f | sed 's#.*/data/##' | sort | tr '\n' ' ')"
host "server: old f7 and f9 kept" '[ "$olds" = "f7 f9 " ]'
grep -aE "^homesync:|^server:|^last push:|^reachable:|^would change:" <<< "$log" | head -20
echo "server keeps: $olds"
[ "$fail" = 0 ] && echo "homesync test: pass" || { echo "homesync test: FAIL, see images/qemu-test.log and $work/sshd.log"; exit 1; }
