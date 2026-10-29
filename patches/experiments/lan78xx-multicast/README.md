# LAN7800 receives all multicast (experiment `lan78xx-multicast`)

For [#27](https://github.com/redox-rpi/redox-rpi/issues/27) (IPv6) and
[#37](https://github.com/redox-rpi/redox-rpi/issues/37) (mDNS) on the Pi 3B+.

- **base `0024`:** `dwc2d` sets the LAN7800's `RFE_CTL_MCAST_EN`, as Linux
  does for all-multicast. Before, the chip took only its own address
  (perfect filter) and broadcast, so it dropped mDNS (`01:00:5e:00:00:fb`,
  `33:33:00:00:00:fb`) and IPv6 neighbor discovery (`33:33:ff:…`). The
  network stack does not tell the driver its groups, so there is no
  per-group hash filter. The RTL815x driver already receives all multicast.
- Applies on `main` (after base `0023`) and on the `usb-hotplug` experiment
  unchanged.

## Tests

- **QEMU:** builds and boots; QEMU has no LAN7800, so this only shows that
  nothing else changed (QEMU's network card is CDC Ethernet).
- **Pi:** with test #47 (`--experiment lan78xx-multicast` added): mDNS
  answers and IPv6 neighbor discovery over the LAN7800.
