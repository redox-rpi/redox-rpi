# Experiment: IPv4 multicast

First step towards IPv6 ([#27](https://github.com/redox-rpi/redox-rpi/issues/27),
v0.6.0): IPv6 neighbor discovery and SLAAC need multicast, and so do mDNS
and SSDP over IPv4. Redox's network stack had none: the Ethernet layer
dropped multicast frames, and relibc skipped `IP_ADD_MEMBERSHIP`.

```bash
scripts/build-image.sh --experiment multicast
```

- **base `0024`, `netstack`:** smoltcp's `multicast` feature on. A
  `setsockopt(IPPROTO_IP, IP_ADD_MEMBERSHIP / IP_DROP_MEMBERSHIP)` joins or
  leaves the group on the interface, which then reports it with IGMP and
  accepts its packets. The Ethernet link lets multicast frames through and
  sends IPv4 multicast to `01:00:5e` plus the low 23 bits of the group,
  without ARP. The router sends multicast on the device of the route to the
  group, never through the route's gateway.
- **relibc `0001`:** `setsockopt` with level `IPPROTO_IP` goes to the
  socket's scheme like `SOL_SOCKET` options did, with the level as a third
  metadata word, so `IP_*` and `SO_*` numbers do not mix. Other `IP_*`
  options (`IP_MULTICAST_TTL`, `IP_MULTICAST_LOOP`, `IP_MULTICAST_IF`) reach
  the stack too, which accepts and ignores them as before.
- **[`tools/mcast`](../../../tools/mcast):** `mcast recv <group> <port>
  [count]` and `mcast send <group> <port> <text> [count]`.

Limits: memberships are counted per scheme (UDP, raw IP) and not left when
a socket closes without `IP_DROP_MEMBERSHIP`; sent multicast does not loop
back to local listeners; at most smoltcp's default of 4 groups. The
LAN7800 driver in `dwc2d` filters multicast frames in hardware, so on a
Pi 3B+ only groups that pass that filter arrive (owner: `dwc2d`); the
RTL8152 driver receives all multicast.

## Test in QEMU

QEMU's user networking drops multicast, so the test uses its socket
netdev: the guest's Ethernet frames travel as UDP datagrams to a host
multicast group, where [`mcast-qemu-test.py`](mcast-qemu-test.py) plays a
second host (10.0.3.2) on the guest's subnet. No DHCP there, so the guest
gets a static address first.

```bash
patches/experiments/multicast/mcast-qemu-test.py --seconds 900 &
scripts/qemu-raspi3b.sh --image <harddrive.img> --usb-net-socket 239.255.77.77:17777 \
    --run 'echo 10.0.3.15/24 > /scheme/netcfg/ifaces/eth0/addr/set' \
    --run 'echo "default via 10.0.3.2" > /scheme/netcfg/route/add' \
    --run 'mcast recv 239.1.2.3 5000 2' \
    --run 'mcast send 239.1.2.3 5001 hello-from-redox 3'
```

Expected: the guest's log shows `mcast: joined 239.1.2.3`, two `hello … from
the host` datagrams and `mcast: left 239.1.2.3`; the script prints an IGMP
report for 239.1.2.3 and the guest's datagrams to `239.1.2.3:5001` with the
MAC `01:00:5e:01:02:03` and ends with `result: igmp yes, udp from guest yes`.

## Result

Not run yet.
