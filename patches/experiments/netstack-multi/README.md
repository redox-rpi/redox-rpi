# Several network adapters (was experiment `netstack-multi`, #53)

**Default since the merge of this branch:** the patches are base
`0041`–`0044` in `patches/base/`. `--experiment netstack-multi` is still
accepted and adds nothing, so older build commands keep working. The
numbers below are the ones the patches had as an experiment: `0050` is now
`0041`, `0051` `0042`, `0052` `0043`, `0053` `0044`. `0043` replaces the
retry loop of base `0032` (dhcpd tries again until the server answers):
an interface without a lease is asked three times, then again every 30 s.

Until now netstack took one adapter at start (`network.*`, the first one
found) and dhcpd configured only `eth0`, once. A second adapter (the Wi-Fi
driver, a USB Ethernet adapter plugged in later) was never used.

| Patch | What |
|---|---|
| `0050` | netstack opens every `network.*` scheme at start and looks again every 2 s. `network.wlan*` become `wlan0`, `wlan1`, …, the others `eth0`, `eth1`, …, wired ones first. `/scheme/netcfg/ifaces` lists them. netstack tells init it is ready only once netcfg and the interfaces exist (with parallel init, dhcpd started too early in Pi test 69). |
| `0051` | A DHCP client broadcast goes out only on the adapter whose MAC is in it, with that interface's address or 0.0.0.0 as source; other broadcasts only on the interface of their source address. After the router changes a packet's source it fills the UDP/TCP checksums again. |
| `0052` | dhcpd keeps running: it reads `ifaces` every 2 s and gets a lease on each new interface (5 s per reply, 3 tries, again after 30 s). It takes only replies with its transaction ID and MAC. One default route: the first wired interface with a lease, else Wi-Fi; the name server comes from the same lease. `dhcpd -i eth0` is the old one-shot mode. |
| `0053` | `testnetd`, a fake adapter for tests (in `/usr/lib/drivers`). |

## Base `0058`: leases renewed, adapters that go away, fallback

On top of `0041`–`0044` (branch `netstack-multi2`):

- **dhcpd renews** a lease at T1 (option 58, else half the lease time,
  option 51), with a broadcast Request that carries the address in
  `ciaddr`. If it gets no answer it asks again every 30 s, and not after the
  lease ends. When the lease ends, or the server sends a NAK, dhcpd drops
  it and starts over with a Discover. A lease without a time, or an
  infinite one, is never renewed.
- **netstack drops an adapter that goes away:** its scheme left `/scheme`,
  or its file cannot be read any more (the driver ended). netstack then
  removes its address, every route through it and the device. If the same
  scheme comes back, it gets its old name again.
- **The default route follows:** when the interface with the default route
  goes away or loses its lease, dhcpd moves the default route to the next
  interface with a lease (wired before Wi-Fi). When none is left, it
  removes the default route. When a preferred interface comes back, the
  default route goes back to it.

Not done: link state. A pulled cable without the adapter going away is
noticed only when the lease ends, because drivers report no carrier.
Restarting a driver under the same scheme name does not work yet: a dead
scheme keeps its name in `/scheme` ("No such device" on open), so
registering it again fails with "File exists" (kernel side).

## testnetd

`/usr/lib/drivers/testnetd [scheme] [lease seconds]` registers
`network.test0` (or the scheme given, e.g. `network.wlan_test0` to get
`wlan0`). Behind it is one host, 10.0.9.1, that answers ARP, ping and DHCP
(it offers 10.0.9.2/24, router and DNS 10.0.9.1, for one hour or the lease
time given). A scheme with `wlan` in its name gets 10.0.8.0/24 and MAC
02:00:00:00:08:02 instead, so two testnetds can run side by side. Each
writes its process ID to `/tmp/<scheme>.pid`; killing it removes the
adapter. While `/tmp/testnetd-nodhcp` exists, DHCP gets no answer.
`/tmp/testnetd.log` holds its counters:

- `dhcp`: DHCP messages for its own MAC (02:00:00:00:09:02)
- `dhcp_foreign`: DHCP messages for another MAC; must stay 0, else another
  interface's DHCP leaked onto it
- `foreign_src`: packets with a source outside 10.0.9.0/24; must stay 0
- `ping`: echo requests answered
- `renew`: Requests that renew a lease (`ciaddr` set)

## Test in QEMU

```bash
scripts/qemu-raspi3b.sh --image <harddrive.img> --usb-net \
  --run '/usr/lib/drivers/testnetd' --run 'sleep 12' \
  --run 'cat /scheme/netcfg/ifaces/eth0/addr/list /scheme/netcfg/ifaces/eth1/addr/list' \
  --run 'cat /scheme/netcfg/route/list' \
  --run 'ping -c 3 10.0.9.1' --run 'ping -c 3 10.0.2.2' \
  --run 'cat /tmp/testnetd.log'
```

Expected: `eth0` (USB CDC Ethernet, 10.0.2.15) and `eth1` (testnetd,
10.0.9.2); the default route stays via 10.0.2.2 on eth0; both pings
answer; `dhcp_foreign 0 foreign_src 0`.

### Leases, an adapter going away, fallback (`0058`)

Without `--usb-net` (`eth0` is then dwc2d's `network.usb`, without a
link):

```bash
T=/usr/lib/drivers/testnetd
SHOW='ls /scheme/netcfg/ifaces; cat /scheme/netcfg/route/list; cat /tmp/testnetd.log'
scripts/qemu-raspi3b.sh --image <harddrive.img> --timeout 900 --run 'sleep 30' \
  --run "$T network.wlan_test0 30" --run 'sleep 20' --run "$SHOW" \
  --run "$T network.test0 30" --run 'sleep 50' --run "$SHOW" \
  --run 'kill $(cat /tmp/network.test0.pid)' --run 'sleep 10' --run "$SHOW" \
  --run 'ping -c 2 10.0.8.1' \
  --run "$T network.test1 30" --run 'sleep 25' --run "$SHOW" \
  --run 'echo x > /tmp/testnetd-nodhcp' --run 'sleep 50' --run "$SHOW" \
  --run 'rm /tmp/testnetd-nodhcp' --run 'sleep 70' --run "$SHOW"
```

Result on `v0.3.0-dev-481-g226ae0a`:

| Step | Serial log and routes |
|---|---|
| wlan0, then eth1 | `DHCP: wlan0 got 10.0.8.2/24`, default via wlan0; `DHCP: eth1 got 10.0.9.2/24`, default moves to eth1 |
| renewal | `DHCP: eth1 renewed 10.0.9.2 for 30 s` and the same for wlan0, every 15 s; `renew` counts up |
| kill eth1's testnetd | `Failed to read ethernet device on link eth1: No such device; dropping it`, `DHCP: eth1 is gone`, default via wlan0, no eth1 routes; ping over wlan0 2/2 |
| new adapter `network.test1` | `eth2`, `DHCP: eth2 got 10.0.9.2/24`, default moves to eth2 |
| DHCP off | `renewal: timed out`, `DHCP: eth2 lease of 10.0.9.2 ended`, default via wlan0; then `wlan0 lease … ended`, `DHCP: no default route` |
| DHCP on | both leases again, default via eth2 |

`cargo test`: dhcpd 4 passed (lease times among them), testnetd 3 passed.
