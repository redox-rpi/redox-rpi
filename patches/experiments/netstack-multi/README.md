# Experiment: netstack-multi, several network adapters (#53)

Until now netstack took one adapter at start (`network.*`, the first one
found) and dhcpd configured only `eth0`, once. A second adapter (the Wi-Fi
driver, a USB Ethernet adapter plugged in later) was never used.

```bash
scripts/build-image.sh --experiment netstack-multi
```

| Patch | What |
|---|---|
| `0050` | netstack opens every `network.*` scheme at start and looks again every 2 s. `network.wlan*` become `wlan0`, `wlan1`, …, the others `eth0`, `eth1`, …, wired ones first. `/scheme/netcfg/ifaces` lists them. |
| `0051` | A DHCP client broadcast goes out only on the adapter whose MAC is in it, other broadcasts only on the interface of their source address. After the router changes a packet's source it fills the UDP/TCP checksums again. |
| `0052` | dhcpd keeps running: it reads `ifaces` every 2 s and gets a lease on each new interface (5 s per reply, 3 tries, again after 30 s). It takes only replies with its transaction ID and MAC. One default route: the first wired interface with a lease, else Wi-Fi; the name server comes from the same lease. `dhcpd -i eth0` is the old one-shot mode. |
| `0053` | `testnetd`, a fake adapter for tests (in `/usr/lib/drivers`). |

Not yet: lease renewal, removing an interface whose adapter goes away, and
a second default route as fallback when the preferred link drops.

## testnetd

`/usr/lib/drivers/testnetd [scheme]` registers `network.test0` (or the
scheme given, e.g. `network.wlan_test0` to get `wlan0`). Behind it is one
host, 10.0.9.1, that answers ARP, ping and DHCP (it offers 10.0.9.2/24,
router and DNS 10.0.9.1). `/tmp/testnetd.log` holds its counters:

- `dhcp`: DHCP messages for its own MAC (02:00:00:00:09:02)
- `dhcp_foreign`: DHCP messages for another MAC; must stay 0, else another
  interface's DHCP leaked onto it
- `foreign_src`: packets with a source outside 10.0.9.0/24; must stay 0
- `ping`: echo requests answered

## Test in QEMU

```bash
scripts/qemu-raspi3b.sh --image <harddrive.img> --usb-net \
  --run '/usr/lib/drivers/testnetd' --run 'sleep 12' \
  --run 'cat /scheme/netcfg/ifaces' --run 'cat /scheme/netcfg/route/list' \
  --run 'ping -c 3 10.0.9.1' --run 'ping -c 3 10.0.2.2' \
  --run 'cat /tmp/testnetd.log'
```

Expected: `eth0` (USB CDC Ethernet, 10.0.2.15) and `eth1` (testnetd,
10.0.9.2); the default route stays via 10.0.2.2 on eth0; both pings
answer; `dhcp_foreign 0 foreign_src 0`.
