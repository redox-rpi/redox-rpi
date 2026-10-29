# Network

## State

- LAN7800 (Pi 3B+ onboard Ethernet) in `dwc2d`: DHCP, TCP from the LAN.
- RTL8152/RTL8153 USB Ethernet (base patch `0020`, since v0.2.0): the
  Zero 2 W gets its time over the HAT's RTL8152.
- `devshd`: remote shell on port 2323, unencrypted, LAN only, never in
  release images.

## Next

- Confirm the Zero 2 W's IP address and TCP (`timesync -n` worked).

## Later

- [#26](https://github.com/redox-rpi/redox-rpi/issues/26) Wi-Fi on the Pi 3B+ and Zero 2 W (milestone v0.6.0); it would be the
  first Wi-Fi in Redox.
- [#27](https://github.com/redox-rpi/redox-rpi/issues/27) IPv6 in `netstack` (milestone v0.6.0); Matter runs over IPv6.
  Analysis (2026-10-28, our pinned `base` and `relibc`):
  - **netstack** does Ethernet and ARP itself (`link/ethernet.rs`) and
    runs smoltcp 0.13 only as an IP-level interface (`Medium::Ip`).
    Turning on `proto-ipv6` gives IPv6 sockets, ICMPv6 and MLD, but not
    neighbor discovery or SLAAC: smoltcp only does those on an Ethernet
    interface. So netstack needs its own neighbor discovery next to ARP,
    must accept IPv6 frames (EtherType 0x86DD) and multicast MAC addresses
    (33:33:…), and IPv6 in the router, route table, `netcfg` and the
    socket paths (`[addr]:port`). The code has about ten places that
    assume IPv4 only. Estimate: one to two weeks.
  - **SLAAC**: smoltcp parses router advertisements, but its SLAAC needs the
    Ethernet interface, so netstack has to send router solicitations and
    form the addresses itself (a few days). Alternative: one smoltcp
    Ethernet interface per device, which brings ARP, neighbor discovery and
    SLAAC from smoltcp, but rewrites netstack's link and router layers and
    moves far from upstream.
  - **DHCPv6**: not in smoltcp, and Redox's `dhcpd` is IPv4 only. Matter
    does not need it; the DNS server can come from the router advertisement
    (RDNSS option). A DHCPv6 client is about a week.
  - **relibc**: `AF_INET6` and `sockaddr_in6` exist and `inet_pton` handles
    IPv6, but `socket()` refuses everything except `AF_INET` and
    `AF_UNIX`, and the address conversions in `bind`, `connect`,
    `accept`, `sendto` and the like are IPv4 only (about a week). DNS asks
    for A records only and `gethostbyname2(AF_INET6)` is a TODO; AAAA
    lookups and IPv6 name servers are a few days.
  - **Multicast** (mDNS, also over IPv4): netstack drops all multicast
    frames today, there are no group joins through relibc
    (`IP_ADD_MEMBERSHIP`, `IPV6_JOIN_GROUP`), and the LAN7800 driver only
    lets its own and broadcast addresses through (the RTL815x driver
    accepts all multicast).
  - **Upstream**: no IPv6 work on netstack. relibc has had small IPv6
    changes in 2026 (`inet_pton`, symbols); its maintainers note that IPv6
    functions are of little use without IPv6 sockets. Changes here stay
    patches in our fork, since Redox takes no AI-generated contributions.
  - Order: multicast and IPv6 in netstack, then relibc sockets and AAAA,
    then SLAAC; all testable in QEMU (`-device usb-net` or virtio).
- SSH and Bluetooth ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
