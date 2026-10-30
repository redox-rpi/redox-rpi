# USB

## State

- `dwc2d`, a DWC2 USB host driver in base: hubs, split transactions for low-
  and full-speed devices behind the Pi 3B+'s hubs, bulk transfers.
- `dwc2d` waits for the controller's interrupt instead of polling (base
  `0022`, kernel `0004`–`0006`), about 5 % of a core instead
  of a full one on a real Pi (test #5,
  [#29](https://github.com/redox-rpi/redox-rpi/issues/29)).
- `dwc2d` asks again for a reply shorter than the fields it reads
  (device descriptor, hub port status, hub descriptor) instead of
  panicking (base `0037`, PR #84; the panic of test 98, Pi test 105
  5 of 5 boots).
- USB keyboard on the HDMI console of a real Pi 3B+ and Zero 2 W.
- Hotplug ([#17](https://github.com/redox-rpi/redox-rpi/issues/17), base
  `0031`–`0033`): dwc2d rescans the root port and all hub ports every
  second. A keyboard or network adapter plugged in later is used, and the
  network moves to the best adapter (RTL815x with a link, else LAN7800,
  else RTL815x without a link, else CDC Ethernet), keeping its MAC address
  and lease. `dhcpd` tries again until the server answers. A test command
  (`/tmp/dwc2d-cycle`) switches a hub port off and on for tests without a
  person (`scripts/test-hotplug.sh` in QEMU, `scripts/pi-hotplug-test.sh`
  on the Pi). Pi tests #35 and #67 ([reports](../test-reports.md)). The
  Anker hub steps stay with test #4.
- Shorter boot (#47, base `0034`): hubs take the ports that are connected
  after power-good instead of waiting for empty ones, and the wait for the
  network link overlaps with rescans for late devices. On the Pi 3B+ dwc2d
  is ready after 3.3 s instead of 6.2 s (2.1 s of it is the LAN7800's
  link), kernel to login 7.4 s instead of 10.3 s (test #67).

## In progress

- [#25](https://github.com/redox-rpi/redox-rpi/issues/25) USB sticks,
  branch [`usb-storage`](https://github.com/redox-rpi/redox-rpi/tree/usb-storage)
  (on `usb-hotplug`), experiment `usb-storage` (base `0038` there).
  `dwc2d` serves a stick (bulk-only transport, SCSI) as the driver-block
  disk scheme `disk.usb-dwc2+<ports>-scsi`; the boot session's `fatfsd`
  mounts it. **QEMU test passes** (`scripts/test-usb-storage.sh`: read,
  write, unplug, plug in again); not yet on the Pi.

- [#48](https://github.com/redox-rpi/redox-rpi/issues/48) USB mouse and
  tablet, branch [`usb-mouse`](https://github.com/redox-rpi/redox-rpi/tree/usb-mouse)
  experiment `usb-mouse` (base `0035` there). Events go
  to inputd like the keyboard's, for Orbital (graphics topic). **QEMU
  passes** (`scripts/test-usb-mouse.sh`); Pi test #56 with a person, same
  visit as #50.
- [#50](https://github.com/redox-rpi/redox-rpi/issues/50) USB serial
  adapters (CDC-ACM, FTDI, CP210x, CH340: Zigbee/Thread sticks, Arduino),
  branch [`usb-serial`](https://github.com/redox-rpi/redox-rpi/tree/usb-serial)
  (on `usb-hotplug`), experiment `usb-serial` (base `0035` there, PR #70). Each is
  `/scheme/usb-serial/<ports>` with a `baud` file and termios calls,
  hotplug. **QEMU passes** (`scripts/test-usb-serial.sh`, 15/15 with
  QEMU's FTDI); Pi test #62 with a person, same visit as #50 and #56.
  relibc's `cfsetospeed` still returns EINVAL on Redox (relibc topic).
- LAN7800 receives all multicast (mDNS, IPv6 neighbor discovery; #27,
  #37), branch [`lan78xx-multicast`](https://github.com/redox-rpi/redox-rpi/tree/lan78xx-multicast),
  experiment `lan78xx-multicast` (base `0024` there). Builds and boots in
  QEMU (no LAN7800 there); the Pi check rides on the network session's
  test #47.

- [#52](https://github.com/redox-rpi/redox-rpi/issues/52) card readers
  (a disk per LUN with a card) and several sticks at once, branch
  [`usb-storage-more`](https://github.com/redox-rpi/redox-rpi/tree/usb-storage-more)
  (base `0039` there, on `usb-storage`). **QEMU passes** (`scripts/test-usb-storage.sh` 18/18, again after the rebase onto the hotplug base patches on 2026-10-30);
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
