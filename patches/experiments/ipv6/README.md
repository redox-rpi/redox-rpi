# Experiment: IPv6 addresses, neighbor discovery, SLAAC

Step 2 of [#27](https://github.com/redox-rpi/redox-rpi/issues/27) (IPv6,
v0.6.0). Applies after the [`multicast`](../multicast) experiment.

```bash
scripts/build-image.sh --experiment multicast --experiment ipv6
```

Base `0025`, `netstack`:

- smoltcp with `proto-ipv6`. Its neighbor discovery and SLAAC only work on
  an Ethernet medium, and netstack runs smoltcp on an IP medium with its own
  Ethernet layer, so that layer now does NDP the way it does ARP:
  - a link-local address `fe80::` + modified EUI-64 of the MAC;
  - answers neighbor solicitations for its own addresses;
  - resolves IPv6 next hops with a solicitation to the solicited-node group;
  - IPv6 multicast to `33:33` + the low 32 bits;
  - up to 3 router solicitations, 4 s apart;
  - from a router advertisement: the router (lifetime above 0) and, for a
    /64 prefix with the autonomous flag, the SLAAC address (RFC 4862).
- Smolnetd copies the link's addresses and router into smoltcp and the
  route table: `fe80::/64`, `ff00::/8`, the prefix, `::/0` via the router,
  and `::1` on the loopback device. `cat /scheme/netcfg/ifaces/eth0/addr/list`
  and `route/list` show them.
- smoltcp now answers echo requests itself (`auto-icmp-echo-reply`), for
  IPv6 **and IPv4**: before, Redox answered no ping at all.

Not done yet: duplicate address detection, address and router lifetimes
(only lifetime 0 removes), more than one prefix, DHCPv6, RDNSS (DNS server
from the advertisement), redirects, static IPv6 addresses through netcfg.
Programs cannot use IPv6 yet: relibc has no `AF_INET6` sockets and the
schemes parse only IPv4 endpoints (step 3).

## Test in QEMU

QEMU's socket netdev, as in the multicast test, with
[`ipv6-qemu-test.py`](ipv6-qemu-test.py) as the router: it answers the
guest's router solicitation with an advertisement for `2001:db8:1::/64`,
asks for the guest's SLAAC address with a neighbor solicitation and pings
the guest's link-local and SLAAC address.

```bash
patches/experiments/ipv6/ipv6-qemu-test.py --seconds 700 &
scripts/qemu-raspi3b.sh --image <harddrive.img> --usb-net-socket 239.255.77.77:17777 \
    --run 'sleep 60' --run 'cat /scheme/netcfg/ifaces/eth0/addr/list' \
    --run 'cat /scheme/netcfg/route/list'
```

## Result

**QEMU** (2026-10-29, build `v0.3.0-dev-148-g36bf9c6+xfa87`): **pass**.
`result: router solicitation yes, neighbor advert for SLAAC address yes,
echo reply link-local yes, echo reply SLAAC yes`. The guest lists
`fe80::5054:ff:fe12:3457/64` and `2001:db8:1:0:5054:ff:fe12:3457/64`, and
the routes `default via fe80::5054:ff:fe12:3499 dev eth0`, `ff00::/8`,
`2001:db8:1::/64` and `fe80::/64` on eth0. In a longer run, 92 and 91 of
the pings to the two addresses were answered, with no loss after the
address was set up.

On the way: a trace build showed the echo requests reaching smoltcp
without an answer, which led to `auto-icmp-echo-reply`. The first echo
reply to the SLAAC address can log `No route found` once, before the route
for the prefix exists (the next poll adds it).

**Real Pi:** not tested yet. On the home network (the Fritz!Box sends
router advertisements): the address list should show a `2a0x:` address,
and `ping -6 <address>` from the build host should answer.
