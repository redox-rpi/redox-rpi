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

- [#104](https://github.com/redox-rpi/redox-rpi/issues/104) no busy-waiting
  in dwc2d: branch [`dwc2d-wait-irq`](https://github.com/redox-rpi/redox-rpi/tree/dwc2d-wait-irq)
  (on `usb-storage-more`), experiment `dwc2d-wait-irq` (base `0071` there).
  Sent frames, stick chunks and control requests spun until the channel
  halted; now they spin 50 µs and then wait for the channel's interrupt on
  a second irq handle, with a 2 ms timer as the fallback. A first try with
  short `sleep`s hung dwc2d in QEMU. **QEMU passes** (storage 18/18,
  hotplug 15/15, keyboard), but sticks are slower there (128 instead of
  182 MB/s), since QEMU finishes a chunk at once; the real check is Pi test
  #135, like for like against #71's image, no person needed.

## Next

- Pi tests of USB sticks and card readers: rows #50 and #71, one image,
  with a person in the USB visit.

## USB gadget mode (effort analysis)

[#95](https://github.com/redox-rpi/redox-rpi/issues/95): the Zero 2 W as a
network adapter on the computer its USB cable is plugged into, without a
HAT. No code yet.

- **What `dwc2d` needs:** device mode next to today's host mode, chosen
  by the device tree (`dr_mode=peripheral` in a `[pi02]` section); the
  device core (FIFOs, device interrupts, endpoint registers); the endpoint
  0 state machine with the standard requests; a CDC-ECM function (Linux,
  macOS) registered as `network.usb-gadget`, CDC-NCM later for Windows.
  The port is host or device, not both. The Pi 3B+ can't do device mode
  (its DWC2 sits behind the LAN7515 hub).
- **Next to `dwc2d` or instead of it:** one controller, so one driver
  owns it. `dwc2d` starts from initfs (`40_dwc2d.service`), finds the
  controller in the device tree by `compatible` and maps its registers;
  Redox's init can't pick a service by a device-tree value, so a separate
  gadget daemon would start next to `dwc2d` and both would take the same
  registers. Better: the same binary reads `dr_mode` from the controller's
  node at start and runs either today's host code or a new device part
  (`device.rs` for the core and endpoint 0, `ecm_gadget.rs` for the
  function), sharing `regs.rs`, the register mapping and the network
  scheme code. The host code stays as it is; nothing changes on the Pi
  3B+ or on a Zero 2 W without the overlay. Switching at run time (OTG ID
  pin, host and device on one port) is not needed and left out.
- **Testing:** QEMU's DWC2 model is host only, so: a host model in Rust
  that plays a host's enumeration against the code, then a Zero 2 W cabled
  to homenodellm (powered by it, `cdc_ether` on the Linux side, tests over
  SSH), booted from SD or, if it works, by rpiboot over the same cable. A
  second USB serial adapter for the Zero's console; one person visit to
  set it up and try a Mac or PC.
- **Size:** about 1700 lines for ECM (half of today's `dwc2d`), about 4–5
  sessions; NCM about 400 lines more. Medium; the risk is the test bench,
  not the code.

## Later

- `lsusb` ([#35](https://github.com/redox-rpi/redox-rpi/issues/35), firmware
  session).

Keyboard layouts are in inputd, not dwc2d:
[#36](https://github.com/redox-rpi/redox-rpi/issues/36), branch
[`t36-keymap`](https://github.com/redox-rpi/redox-rpi/tree/t36-keymap)
(t18 session), experiment `keyboard-de`: German layout with AltGr and dead
keys, set at boot from `/etc/keymap`. QEMU passes; Pi test #150 with a
person and a German USB keyboard.
