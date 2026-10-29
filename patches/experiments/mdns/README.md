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
- **IPv6** (branch `mdns6`, needs the [`ipv6`](../ipv6) experiment):
  the same socket joins `ff02::fb`, answers AAAA queries with every IPv6
  address of `eth0` (link-local and SLAAC), sends answers to the group of
  the family the query came in on, adds the other family's address records
  as additional records (RFC 6762 section 6.2) and announces on both
  groups. Without IPv6 in netstack it logs that and stays IPv4 only.
- [`mdns-query.py`](mdns-query.py): one-shot query from the build host
  (`mdns-query.py redox-rpi.local AAAA -6 wlp13s0`), for the Pi tests:
  homenodellm has no avahi.
- `config.toml`: hostname `redox-rpi` (`/etc/hostname`) and the service
  `91_mdnsd.service`, which announces devsh. devshd is for development
  images only, so a release image would announce other services.

Not done yet: probing for name conflicts (RFC 6762 section 8), goodbye
packets at shutdown, known-answer suppression. On a Pi 3B+, the LAN7800
multicast filter in `dwc2d` must pass `01:00:5e:00:00:fb` and
`33:33:00:00:00:fb`: experiment `lan78xx-multicast` (dwc2d session).

## Test in QEMU

Like the multicast test: QEMU's socket netdev, and
[`mdns-qemu-test.py`](mdns-qemu-test.py) as a second host (10.0.3.2) that
asks for `redox-rpi.local` (A, from port 5353 and from port 40000) and
`_devsh._tcp.local` (PTR), and answers ARP.

```bash
patches/experiments/mdns/mdns-qemu-test.py --seconds 850 &   # --ipv6: AAAA over IPv6 too
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

**QEMU, IPv6** (2026-10-29, branch `mdns6`, build
`v0.3.0-dev-160-gd2c8e48+x919d`, `--experiment multicast --experiment ipv6
--experiment mdns`): **pass**, `mdns-qemu-test.py --ipv6`: `result:
multicast A yes, unicast A yes, SRV 2323 yes, multicast AAAA over IPv6 yes,
unicast AAAA over IPv6 yes`. `mdnsd: redox-rpi.local is 10.0.3.15,
fe80::5054:ff:fe12:3457`; the answer to the AAAA query from port 5353 went
to `[ff02::fb]:5353`, the one from port 40000 to
`[fe80::5054:ff:fe12:3499]:40000` with its ID, both with the A record as
additional record; IPv4 answers carry the AAAA record the same way.

**Real Pi:** not tested yet. Test #47 (step 2 of IPv6, IPv4 mDNS) and a
second test for this branch are in the [queue](../../../docs/test-plan.md).
