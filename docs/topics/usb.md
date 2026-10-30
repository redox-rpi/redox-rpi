# USB

## State

- `dwc2d`, a DWC2 USB host driver in base: hubs, split transactions for low-
  and full-speed devices behind the Pi 3B+'s hubs, bulk transfers.
- `dwc2d` waits for the controller's interrupt instead of polling (base
  `0022`, kernel `0004`–`0006`), about 5 % of a core instead
  of a full one on a real Pi (test #5,
  [#29](https://github.com/redox-rpi/redox-rpi/issues/29)).
- netstack finds `dwc2d`'s network adapter every boot: `initnsmgr`
  lists `/scheme` from a sorted snapshot, so a scheme registered meanwhile
  (`sudo` at boot) no longer hides `network.*` (base `0036`, PR #80,
  [#79](https://github.com/redox-rpi/redox-rpi/issues/79); 1 in 36 QEMU
  boots before, Pi test 100 3 of 3).
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
  port off and on for tests without a person. QEMU: 15 of 15 checks of
  `scripts/test-hotplug.sh` pass. Pi test #35 is queued: part A automated
  with `scripts/pi-hotplug-test.sh`, part B with a person, bundled with
  the Anker hub test #4.
- `dwc2-default`: RTL8152 and dwc2 interrupts as default patches, merged
  with main; first in the rc2 merge order, waits for the Pi test.

- [#25](https://github.com/redox-rpi/redox-rpi/issues/25) USB sticks,
  branch [`usb-storage`](https://github.com/redox-rpi/redox-rpi/tree/usb-storage)
  (on `usb-hotplug`), experiment `usb-storage` (base `0033` there).
  `dwc2d` serves a stick (bulk-only transport, SCSI) as the driver-block
  disk scheme `disk.usb-dwc2+<ports>-scsi`; the boot session's `fatfsd`
  mounts it. **QEMU test passes** (`scripts/test-usb-storage.sh`: read,
  write, unplug, plug in again); not yet on the Pi.

- [#48](https://github.com/redox-rpi/redox-rpi/issues/48) USB mouse and
  tablet, branch [`usb-mouse`](https://github.com/redox-rpi/redox-rpi/tree/usb-mouse)
  (on `usb-hotplug`), experiment `usb-mouse` (base `0035` there). Events go
  to inputd like the keyboard's, for Orbital (graphics topic). **QEMU
  passes** (`scripts/test-usb-mouse.sh`); Pi test #56 with a person, same
  visit as #50.
- [#50](https://github.com/redox-rpi/redox-rpi/issues/50) USB serial
  adapters (CDC-ACM, FTDI, CP210x, CH340: Zigbee/Thread sticks, Arduino),
  branch [`usb-serial`](https://github.com/redox-rpi/redox-rpi/tree/usb-serial)
  (on `usb-hotplug`), experiment `usb-serial` (base `0036` there). Each is
  `/scheme/usb-serial/<ports>` with a `baud` file and termios calls,
  hotplug. **QEMU passes** (`scripts/test-usb-serial.sh`, 15/15 with
  QEMU's FTDI); Pi test #62 with a person, same visit as #50 and #56.
  relibc's `cfsetospeed` still returns EINVAL on Redox (relibc topic).
- LAN7800 receives all multicast (mDNS, IPv6 neighbor discovery; #27,
  #37), branch [`lan78xx-multicast`](https://github.com/redox-rpi/redox-rpi/tree/lan78xx-multicast),
  experiment `lan78xx-multicast` (base `0024` there). Builds and boots in
  QEMU (no LAN7800 there); the Pi check rides on the network session's
  test #47.

- `dwc2d` boot time (#47): branch [`dwc2d-boottime`](https://github.com/redox-rpi/redox-rpi/tree/dwc2d-boottime)
  (on `usb-hotplug`), experiment `dwc2d-boottime` (base `0037` there).
  Test 53 measured 6.2 s until dwc2d is ready on the Pi: 1.2 s per hub
  waiting for empty ports, 1.7 s for the LAN7800's link. Hubs now take
  what is connected, and the link wait overlaps with rescans. QEMU: ready
  after 0.5 s instead of 1.6 s, `scripts/test-hotplug.sh` 15/15. **Pi test
  #67 passes**: ready after 3.3 s instead of 6.2 s (2.1 s of it is the
  LAN7800's link), kernel to login 7.4 s instead of 10.3 s, hotplug 15/15.
  Merges with `usb-hotplug` once #35 passes.

- [#52](https://github.com/redox-rpi/redox-rpi/issues/52) card readers
  (a disk per LUN with a card) and several sticks at once, branch
  [`usb-storage-more`](https://github.com/redox-rpi/redox-rpi/tree/usb-storage-more)
  (base `0038`). **QEMU passes** (`scripts/test-usb-storage.sh` 18/18);
  Pi test #71 with a person, same visit as #50.

## Next

- A Pi test of USB sticks (high-speed stick on the 3B+ hub).

## Later

- `lsusb` ([#35](https://github.com/redox-rpi/redox-rpi/issues/35), firmware
  session).

Keyboard layouts are in inputd, not dwc2d:
[#36](https://github.com/redox-rpi/redox-rpi/issues/36), branch
[`keyboard-layouts`](https://github.com/redox-rpi/redox-rpi/tree/keyboard-layouts)
(t18 session), Pi test #64 with a person.
