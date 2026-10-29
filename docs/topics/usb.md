# USB

## State

- `dwc2d`, a DWC2 USB host driver in base: hubs, split transactions for low-
  and full-speed devices behind the Pi 3B+'s hubs, bulk transfers.
- `dwc2d` waits for the controller's interrupt instead of polling (base
  `0022`, kernel `0004`–`0006`), about 5 % of a core instead
  of a full one on a real Pi (test #5,
  [#29](https://github.com/redox-rpi/redox-rpi/issues/29)).
- USB keyboard on the HDMI console of a real Pi 3B+ and Zero 2 W, when
  plugged in before boot.

## In progress

- [#17](https://github.com/redox-rpi/redox-rpi/issues/17) hotplug, branch
  [`usb-hotplug`](https://github.com/redox-rpi/redox-rpi/tree/usb-hotplug)
  (on `dwc2-default`), experiment `usb-hotplug`: base `0030` rescans the root port and all hub ports
  every second; a keyboard or network adapter plugged in later is used,
  and the network moves to the best adapter (RTL815x with a link, else
  LAN7800, else RTL815x without a link, else CDC Ethernet), keeping its MAC
  address and lease. Base `0031`: `dhcpd` tries again until the server
  answers. QEMU: `scripts/test-hotplug.sh`. Pi test queued, bundled with
  the Anker hub test (#4).

## Next

- [#25](https://github.com/redox-rpi/redox-rpi/issues/25) USB sticks on
  `dwc2d` (bulk-only transport, SCSI), with the Bootloader? session for the
  FAT side.

## Later

- Keyboard layouts, `lsusb`
  ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
