# Experiment: lan78xx-mcast-hash

Issue [#216](https://github.com/redox-rpi/redox-rpi/issues/216), for the
idle power work of [#126](https://github.com/redox-rpi/redox-rpi/issues/126).

Base patch `0084` (from #194) makes the LAN7800 accept **all** multicast,
because the network stack doesn't tell the driver which groups it joined.
In Pi test #246 that added about 28 wakeups a second to both `dwc2d` and
`netstack`, plus 10 USB interrupts a second, for mDNS, SSDP and IPv6
frames that the IPv4-only stack then drops.

## Patch `0095`: hash filter in `lan78xx.rs`

`lan78xx::set_multicast(bus, f, groups)`:

- writes the chip's 512-bit multicast hash table through the `DP_SEL`
  data port, at word 128 of the VLAN/DA RAM, as Linux's
  `lan78xx_set_multicast` does;
- `Some(groups)`: switches `RFE_CTL` from `MCAST_EN` (all multicast) to
  `MCAST_HASH`, so only frames whose address hashes to a set bit pass.
  The hash is the top 9 bits of the Ethernet CRC (Linux `ether_crc`);
- `Some(&[])`: no multicast at all, only the own address and broadcast;
- `None`: all multicast, as now.

A hash filter can let a few other groups through (512 bits), but nothing
like the whole LAN.

## Wiring (base `0085`, PR #218)

The Network session's base patch `0085` (branch `mcast-filter`, merged
into this branch) adds `NetworkAdapter::set_multicast` and a write-only
`multicast` file to `driver-network`. netstack writes all-hosts
(01:00:5e:00:00:01) plus each joined IPv4 group, at start and on every
change.

In `dwc2d`, `UsbNet` keeps that list. The event loop has the bus, so it
programs the LAN7800 in use after the scheme event and whenever another
adapter takes over. It logs `dwc2d: LAN7800 multicast filter: N groups`.
Until netstack writes a list, all multicast passes as before. The
RTL815x and QEMU's CDC Ethernet are unchanged: they still receive all
multicast.

Not for the default yet: the `ipv6` experiment has to add its 33:33
groups to the list first, or IPv6 neighbor discovery breaks under the
hash filter (Network session, #216).

## Test

QEMU has no LAN7800, so QEMU only checks that the image boots, the
network works over CDC Ethernet, and netstack's write to `multicast`
doesn't fail. The real check is on the Pi:

- the boot log has `dwc2d: LAN7800 multicast filter: 1 groups` (or
  more);
- `cputop -w`: `dwc2d` and `netstack` wakeups should drop to about the
  level from before #194 (Pi test #246: 80/s and 30/s, against 110/s and
  59/s with all multicast);
- mDNS and the network still work.
