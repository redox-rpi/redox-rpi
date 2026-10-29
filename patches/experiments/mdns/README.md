# Experiment: mDNS responder

[#37](https://github.com/redox-rpi/redox-rpi/issues/37), v0.8.0: the Pi
answers as `redox-rpi.local` and announces its services with DNS-SD, so
homenode, Matter and people find it without a fixed address.

```bash
scripts/build-image.sh --experiment multicast --experiment mdns
```

Needs the [`multicast`](../multicast) experiment first (joining
224.0.0.251 with `IP_ADD_MEMBERSHIP`).

- [`tools/mdnsd`](../../../tools/mdnsd): answers A queries for
  `<hostname>.local` with the address of `eth0`, and PTR, SRV and TXT
  queries for the services given on the command line
  (`mdnsd _devsh._tcp:2323`), plus `_services._dns-sd._udp.local`.
  Announces all records twice at start and whenever the address changes.
  Queries from port 5353 get a multicast answer, one-shot queries from
  other ports a unicast answer with their ID and question.
- `config.toml`: hostname `redox-rpi` (`/etc/hostname`) and the service
  `91_mdnsd.service`, which announces devsh. devshd is for development
  images only, so a release image would announce other services.

Not done yet: probing for name conflicts (RFC 6762 section 8), goodbye
packets at shutdown, known-answer suppression, AAAA records (with IPv6,
#27). On a Pi 3B+, the LAN7800 multicast filter in `dwc2d` must pass
224.0.0.251 (`01:00:5e:00:00:fb`).

## Test in QEMU

Like the multicast test: QEMU's socket netdev, and
[`mdns-qemu-test.py`](mdns-qemu-test.py) as a second host (10.0.3.2) that
asks for `redox-rpi.local` (A, from port 5353 and from port 40000) and
`_devsh._tcp.local` (PTR), and answers ARP.

```bash
patches/experiments/mdns/mdns-qemu-test.py --seconds 850 &
scripts/qemu-raspi3b.sh --image <harddrive.img> --usb-net-socket 239.255.77.77:17777 \
    --run 'echo 10.0.3.15/24 > /scheme/netcfg/ifaces/eth0/addr/set' --run 'sleep 40'
```

## Result

**QEMU** (2026-10-29, build `v0.3.0-dev-144-g0a89beb+x1914`): **pass**.
`mdnsd: redox-rpi.local is 10.0.3.15`, then two announcements with the A,
PTR, SRV and TXT records; a multicast answer for the A question, the PTR
answer with the A record as additional record, and a unicast answer to
10.0.3.2:40000 with ID 0x4242. The script ended with `result: multicast A
yes, unicast A yes, SRV 2323 yes`.

The first run failed: without a default route (static address, no DHCP)
the stack had no route for 224.0.0.251 and dropped every answer. The
multicast experiment now adds `224.0.0.0/4 dev eth0` when eth0 gets its
address.

**Real Pi:** not tested yet. Then from the build host: `ping
redox-rpi.local`, `avahi-resolve -n redox-rpi.local`, `avahi-browse -r
_devsh._tcp`.
