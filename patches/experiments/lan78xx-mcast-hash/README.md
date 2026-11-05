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

## Still to do

- **Wiring:** the Network session adds a `multicast` file and a
  `NetworkAdapter::set_multicast` method to `driver-network` (branch
  `mcast-filter`), and netstack writes 01:00:5e:00:00:01 plus each joined
  group. Patch `0095` then implements that method for the LAN7800.
- QEMU has no LAN7800, so the check is on the Pi: with `cputop -w` the
  `dwc2d` and `netstack` wakeups should drop to about the level from
  before #194 (Pi test #246: 80/s and 30/s), and mDNS still works.
