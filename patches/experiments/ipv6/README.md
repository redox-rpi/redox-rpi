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
  - from a router advertisement: the router (lifetime above 0) and, for
    each /64 prefix with the autonomous flag, a SLAAC address (RFC 4862).
    Since base `0029` every prefix counts (smoltcp keeps only the last
    one, so with the Fritz!Box's global and unique local prefix only the
    unique local address came up, Pi tests #47 and #55); global addresses
    come first and are the source off-link, and a router lifetime of 0
    ends only the router that sent it.
- Smolnetd copies the link's addresses and router into smoltcp and the
  route table: `fe80::/64`, `ff00::/8`, the prefix, `::/0` via the router,
  and `::1` on the loopback device. `cat /scheme/netcfg/ifaces/eth0/addr/list`
  and `route/list` show them.
- smoltcp now answers echo requests itself (`auto-icmp-echo-reply`), for
  IPv6 **and IPv4**: before, Redox answered no ping at all.

Step 3, base `0026`–`0027` and relibc `0002`, so programs can use IPv6:

- **Sockets:** socket paths take and show IPv6 endpoints in brackets
  (`udp:[2001:db8:1::1]:7/[…]:49168`). relibc opens `AF_INET6` sockets on
  the same `tcp` and `udp` schemes, converts `sockaddr_in6` both ways, and
  sends IPv4-mapped addresses (`::ffff:a.b.c.d`) as IPv4. So std's
  `UdpSocket::bind("[::]:0")`, `send_to` and `recv_from` work over IPv6.
- **DNS server from the advertisement:** the RDNSS option (RFC 8106) is
  read by netstack (smoltcp does not parse it) and shown in
  `/scheme/netcfg/resolv/nameserver6`.
- **Resolver:** `getaddrinfo` asks for AAAA records too (A first for
  `AF_UNSPEC`, only AAAA for `AF_INET6`), accepts numeric IPv6 hosts and
  returns `sockaddr_in6`. It tries each name server in turn: those in
  `/etc/net/dns` (IPv4 or IPv6), then those from `nameserver6`, with a 5 s
  timeout each, and closes its socket (it leaked one per lookup before).
- **Socket timeouts for recvmsg** (`0027`): `SO_RCVTIMEO`/`SO_SNDTIMEO`
  applied only to plain reads and writes; `recvfrom` with an address (as
  std's `recv_from` does) is a `recvmsg` call and waited forever. That is
  an upstream bug, not IPv6 specific.
- **IPv6 multicast for programs** (step 4, base `0028`, relibc `0003`):
  `IPV6_JOIN_GROUP`/`IPV6_LEAVE_GROUP` join groups on the interface,
  counted like the IPv4 ones, and smoltcp sends the MLDv2 reports. relibc
  passes `IPPROTO_IPV6` options to netstack like `IPPROTO_IP`; netstack
  accepts and ignores the others (`IPV6_V6ONLY`, `IPV6_MULTICAST_HOPS`,
  ...). Tested through `mdnsd` on branch `mdns6`.
- `udp-echo.py`: UDP echo on the build host for the Pi test
  (`net6 udp [<host>]:7777 hello`).
- `tools/net6`: `net6 resolve <name>`, `net6 udp <[addr]:port> <text>`,
  `net6 tcp <[addr]:port> <text>`, all through std and relibc.

Not done yet: duplicate address detection, address and router lifetimes
(only lifetime 0 removes), DHCPv6, redirects, static
IPv6 addresses through netcfg, scope IDs (`%eth0`; there is only one
interface), `gethostbyname2(AF_INET6)`, `getnameinfo` for IPv6,
`IPV6_PKTINFO` (which address a packet came to), `IPV6_V6ONLY` (sockets
take both families anyway). The DHCP name server still only goes to
`/scheme/netcfg/resolv/nameserver`, which relibc does not read (it reads
`/etc/net/dns`, 9.9.9.9 by default): an older gap, not changed here.

## Test in QEMU

QEMU's socket netdev, as in the multicast test, with
[`ipv6-qemu-test.py`](ipv6-qemu-test.py) as the router: it answers the
guest's router solicitation with an advertisement for `2001:db8:1::/64`,
asks for the guest's SLAAC address with a neighbor solicitation and pings
the guest's link-local and SLAAC address.

With `--udp` the script also names itself as DNS server (RDNSS), answers
`matter.test` AAAA `2001:db8:1::99` and echoes UDP to port 7.

With `--ula` it acts like the Fritz!Box of Pi tests #47 and #55: the
advertisement carries `fd00:db8:2::/64` after the global prefix, and a
second router (`fe80::5054:ff:fe12:3498`) announces only that prefix
with router lifetime 0. Checks: an echo reply from the guest's ULA
address, and an echo reply to `2001:db8:99::1` (off-link, so it needs
the default route) sent to the first router's MAC.

```bash
patches/experiments/ipv6/ipv6-qemu-test.py --udp --seconds 700 &
scripts/qemu-raspi3b.sh --image <harddrive.img> --usb-net-socket 239.255.77.77:17777 \
    --run 'sleep 60' --run 'cat /scheme/netcfg/ifaces/eth0/addr/list' \
    --run 'cat /scheme/netcfg/route/list' \
    --run 'net6 udp [2001:db8:1::1]:7 hello' --run 'net6 resolve matter.test'
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

**QEMU, step 3** (2026-10-29, build `v0.3.0-dev-153-ga03dcd4+xab49`):
**pass**, all six checks of `--udp`. `nameserver6` shows `2001:db8:1::1`;
`net6 udp [2001:db8:1::1]:7 hello` prints `udp reply from
[2001:db8:1::1]:7: hello` within a second; `net6 resolve matter.test`
prints `matter.test: 2001:db8:1::99` after 12 s, because there is no IPv4
network in this test and the query to 9.9.9.9 times out twice (A and AAAA)
before the IPv6 server answers. The first build hung in that spot, which
is how the missing `recvmsg` timeout showed up. Not tested: UDP to a
link-local address (the script had already quit), TCP over IPv6.

**Real Pi:** not tested yet. On the home network (the Fritz!Box sends
router advertisements): the address list should show a `2a0x:` address,
and `ping -6 <address>` from the build host should answer.
