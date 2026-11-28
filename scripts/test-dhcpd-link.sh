#!/usr/bin/env bash
# QEMU test for dhcpd on link state (#764, RFC 0004 step 1): link down
# undoes the lease, link up gets a new one, nothing happens in between.
#
# Boots the image in QEMU raspi3b without a network card and uses testnetd
# (base 0044, 0130), a fake adapter with a DHCP server whose link a test can
# pull: `echo 0 > /tmp/testnetd.link`. A second dhcpd runs with a
# configuration that names only its interface test0 (the boot dhcpd keeps
# eth0, which QEMU does not have). Checks: lease, default route and status
# line; after link down no address, no route through test0, status "no
# link" and no dhcpd line for 30 s; after link up a new lease and route.
#
# Needs an image from scripts/build-image.sh (default image).
#
# Usage: scripts/test-dhcpd-link.sh [--timeout <seconds>]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
timeout=600
[ "${1:-}" = "--timeout" ] && timeout="$2"

N=/scheme/netcfg
rc=0
scripts/qemu-raspi3b.sh --test --timeout "$timeout" \
    --run "sleep 5" \
    --run "echo '[interface.test0]' > /etc/net/config.toml" \
    --run "echo 'ipv4 = \"dhcp\"' >> /etc/net/config.toml" \
    --run "dhcpd -v &> /tmp/d.log &" \
    --run "/usr/lib/drivers/testnetd" \
    --run "sleep 15" \
    --run "echo UP1-ADDR \$(cat $N/ifaces/test0/addr/list)" \
    --run "cat $N/route/list" \
    --run "echo UP1-STATUS \$(grep test0 /tmp/dhcpd.status)" \
    --run "echo 0 > /tmp/testnetd.link" \
    --run "sleep 5" \
    --run "echo DOWN-ADDR \$(cat $N/ifaces/test0/addr/list)" \
    --run "echo DOWN-ROUTES \$(grep -c test0 $N/route/list)" \
    --run "echo DOWN-DEFAULT \$(grep -c default $N/route/list)" \
    --run "echo DOWN-STATUS \$(grep test0 /tmp/dhcpd.status)" \
    --run "echo DOWN-LINES-A \$(wc -l < /tmp/d.log)" \
    --run "sleep 30" \
    --run "echo DOWN-LINES-B \$(wc -l < /tmp/d.log)" \
    --run "echo 1 > /tmp/testnetd.link" \
    --run "sleep 15" \
    --run "echo UP2-ADDR \$(cat $N/ifaces/test0/addr/list)" \
    --run "echo UP2-DEFAULT \$(grep default $N/route/list)" \
    --run "echo UP2-STATUS \$(grep test0 /tmp/dhcpd.status)" \
    --run "cat /tmp/d.log" \
    > /dev/null 2>&1 || rc=$?
log="$(tr -d '\r' < images/qemu-test.log | sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g;s/\x1b\][^\x07]*\x07//g')"
[ "$rc" = 0 ] || { echo "QEMU run failed (rc $rc), see images/qemu-test.log" >&2; exit 1; }

fail=0
check() { # <name> <extended regex on a whole line>
    if grep -aqE "^$2\$" <<< "$log"; then echo "pass  $1"; else echo "FAIL  $1 (no line matching $2)"; fail=1; fi
}
check "lease on link up"              "UP1-ADDR 10\.0\.9\.2/24"
check "default route via test0"       "default via 10\.0\.9\.1 dev test0 .*"
check "status bound"                  "UP1-STATUS test0: bound 10\.0\.9\.2/24 via 10\.0\.9\.1"
check "log: address removed"          "DHCP: test0 link down, 10\.0\.9\.2/24 and its routes removed"
check "link down: no address"         "DOWN-ADDR Not configured"
check "link down: no route via test0" "DOWN-ROUTES 0"
check "link down: no default route"   "DOWN-DEFAULT 0"
check "link down: status no link"     "DOWN-STATUS test0: no link"
a="$(grep -aoE '^DOWN-LINES-A [0-9]+' <<< "$log" | cut -d' ' -f2)"
b="$(grep -aoE '^DOWN-LINES-B [0-9]+' <<< "$log" | cut -d' ' -f2)"
if [ -n "$a" ] && [ "$a" = "$b" ]; then echo "pass  no dhcpd line in 30 s without link"; else echo "FAIL  dhcpd lines without link ($a -> $b)"; fail=1; fi
check "new lease on link up again"    "UP2-ADDR 10\.0\.9\.2/24"
check "default route back"            "UP2-DEFAULT default via 10\.0\.9\.1 dev test0 .*"
check "status bound again"            "UP2-STATUS test0: bound 10\.0\.9\.2/24 via 10\.0\.9\.1"
grep -aE "^(DHCP|dhcpd):" <<< "$log" | head -20
[ "$fail" = 0 ] && echo "dhcpd link test: pass" || { echo "dhcpd link test: FAIL, see images/qemu-test.log"; exit 1; }
