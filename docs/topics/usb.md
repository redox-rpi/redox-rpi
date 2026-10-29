# USB

## State

- `dwc2d`, a DWC2 USB host driver in base: hubs, split transactions for low-
  and full-speed devices behind the Pi 3B+'s hubs, bulk transfers.
- Experiment `dwc2-interrupts` (with `rtl8152`): `dwc2d` waits for the
  controller's interrupt instead of polling, about 5 % of a core instead
  of a full one on a real Pi (test #5,
  [#29](https://github.com/redox-rpi/redox-rpi/issues/29)).
- USB keyboard on the HDMI console of a real Pi 3B+ and Zero 2 W, when
  plugged in before boot.

## In progress

- [#17](https://github.com/redox-rpi/redox-rpi/issues/17) hotplug, branch
  [`usb-hotplug`](https://github.com/redox-rpi/redox-rpi/tree/usb-hotplug),
  experiment `usb-hotplug` on top of `dwc2-default`. dwc2d rescans the
  root port and all hub ports every second. A keyboard or network adapter
  plugged in later is used, and the network moves to the best adapter
  (RTL815x with a link, else LAN7800, else RTL815x without a link, else CDC
  Ethernet), keeping its MAC address and lease. `dhcpd` tries again until
  the server answers. A test command (`/tmp/dwc2d-cycle`) switches a hub
  port off and on for tests without a person. QEMU: 16 of 16 checks of
  `scripts/test-hotplug.sh` pass. Pi test #35 is queued: part A automated
  with `scripts/pi-hotplug-test.sh`, part B with a person, bundled with
  the Anker hub test #4.
- `dwc2-default`: RTL8152 and dwc2 interrupts as default patches, merged
  with main; first in the rc2 merge order, waits for the Pi test.

## Next

- [#25](https://github.com/redox-rpi/redox-rpi/issues/25) USB sticks on
  `dwc2d` (bulk-only transport, SCSI) as a driver-block disk scheme
  `disk.usb-dwc2+<port>-scsi`; the boot session's `fatfsd` mounts it.

## Later

- [#25](https://github.com/redox-rpi/redox-rpi/issues/25) USB sticks (mass storage), milestone v0.5.0 or v0.6.0.
- Keyboard layouts, `lsusb`
  ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
