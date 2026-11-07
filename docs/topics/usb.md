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
  tablet, branch [`t48-mouse`](https://github.com/redox-rpi/redox-rpi/tree/t48-mouse)
  (on main, PR #149), base `0059`, a default patch since PR #223
  (was experiment `usb-mouse`). Events go
  to inputd like the keyboard's, for Orbital (graphics topic). **QEMU
  passes** (`scripts/test-usb-mouse.sh` 18/18); Pi test #56 with a person,
  same visit as #50.
- [#50](https://github.com/redox-rpi/redox-rpi/issues/50) USB serial
  adapters (CDC-ACM, FTDI, CP210x, CH340: Zigbee/Thread sticks, Arduino),
  branch [`usb-serial`](https://github.com/redox-rpi/redox-rpi/tree/usb-serial)
  (on `usb-hotplug`), base `0035` (PR #70, waits for #62). Each is
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

- [#151](https://github.com/redox-rpi/redox-rpi/issues/151) the Pi's
  inner USB hub (LAN7515, with the LAN7800 and the keyboard) dropped off
  during a download into the SD card, once with dwc2d stuck afterwards:
  branch [`usb-hub-drop`](https://github.com/redox-rpi/redox-rpi/tree/usb-hub-drop)
  (on main, PR #156), experiments `dwc2d-netstats` (`0052`) and
  `usb-hub-drop` (`0083`). No `sleep` in dwc2d any more (a `nanosleep`
  could hang it); a removal logs the hub port's status and change bits and
  the channel errors; a disabled or unpowered port is brought back, a
  failed device gets three more tries; counters in `/tmp/dwc2d-usbstats`,
  the hotplug step in `/tmp/dwc2d-step`. **Pi: 0 hub drops in 4 downloads**
  (tests [172](https://github.com/redox-rpi/redox-rpi/blob/main/docs/test-reports.md#2026-10-31-v030-dev-857-g7ed9070xa901-dwc2d-hub-drop-fix-the-171-download-again),
  [174](https://github.com/redox-rpi/redox-rpi/blob/main/docs/test-reports.md#2026-10-3111-01-hub-drop-fix-v030-dev-857-g7ed9070xa901-new-against-v030-dev-838-g64fa695x8116-old-alternating)),
  against 8, 12 and 4 on the old image (171, 174); no undervoltage. Why
  the drops stop without `sleep` is not known. The download stalls that
  remain are a lost wakeup in the scheduler (#181, relibc topic), not
  dwc2d.
  A different case came in test 189: the controller itself got stuck
  (channels that do not halt, every request timing out). Branch
  [`usb-restart`](https://github.com/redox-rpi/redox-rpi/tree/usb-restart)
  (on `usb-hub-drop`), experiment `dwc2d-restart` (`0084`): after four
  timeouts in a row dwc2d logs the registers, resets the controller, powers
  the root port and finds the devices again (up to 5 tries, 10 s apart).
  **Pi 191: forced restarts recover**; a real stall did not come in 6
  downloads.

## Next

- Pi tests of USB sticks and card readers: rows #50 and #71, one image,
  with a person in the USB visit.

## USB gadget mode (effort analysis)

[#95](https://github.com/redox-rpi/redox-rpi/issues/95): the Zero 2 W as a
network adapter on the computer its USB cable is plugged into, without a
HAT. Analysis only, no code yet (updated 2026-11-04).

- **What `dwc2d` needs:** device mode next to today's host mode, chosen
  by the device tree (`dr_mode=peripheral` in a `[pi02]` section); the
  device core (FIFO sizes `GRXFSIZ`/`GNPTXFSIZ`/`DIEPTXFn`, `DCFG`/`DCTL`
  for address and soft connect, the device interrupts, the endpoint
  registers `DIEPCTLn`/`DOEPCTLn`/`DIEPTSIZn`/`DOEPTSIZn`/`DIEPDMAn`/
  `DOEPDMAn`, buffer DMA as in host mode); the endpoint 0 state machine
  with the standard requests; a network function with two bulk endpoints
  and one interrupt endpoint. The port is host or device, not both. The
  Pi 3B+ can't do device mode (its DWC2 sits behind the LAN7515 hub).
- **One driver owns the controller:** `dwc2d` starts from initfs, and
  Redox's init can't pick a service by a device-tree value, so a separate
  gadget daemon would grab the same registers. The same binary reads
  `dr_mode` at start and runs either today's host code or a new device
  part (`device.rs`: core and endpoint 0; `gadget_net.rs`: the function),
  sharing `regs.rs`, the register mapping and the network scheme code.
  Nothing changes on the Pi 3B+ or on a Zero 2 W without the overlay.
- **Which class:**

  | Class | Linux | macOS | Windows |
  |---|---|---|---|
  | CDC-ECM | built in (`cdc_ether`) | built in | no driver from Microsoft |
  | CDC-NCM | built in (`cdc_ncm`) | built in | built in on 11; on 10 since 1903 (`UsbNcm`), bound without an `.inf` only with Microsoft OS descriptors (`WINNCM`) |
  | RNDIS | built in (its removal has been proposed) | no | built in, but an old Microsoft-only protocol |

  So **NCM is the target**: one class for all three. ECM goes first
  because it is simpler (one Ethernet frame per bulk transfer, no NTB
  framing) and enough for the Linux test bench; NCM adds the NTB16
  framing and the Microsoft OS 2.0 descriptors. RNDIS is left out.
- **Into netstack:** the device part is one more adapter in `dwc2d`'s
  `UsbNet` (`Adapter::Gadget`), registered as `network.usb_gadget` like
  `network.usb_ecm`; netstack already opens every `network.*` scheme, so it
  needs no change. `read_packet`/`write_packet` take frames from the bulk
  OUT endpoint and queue them on bulk IN; the interrupt endpoint reports
  link up when the host selects the data interface. Addresses: the computer
  usually wants a DHCP server on the gadget, and Redox has only the client.
  The gadget gets a fixed address (`192.168.7.1/24`, as on BeagleBone) and
  a small DHCP server that gives the computer `192.168.7.2`; `dhcpd` must
  not run as a client on that adapter. mDNS (`redox-rpi.local`, PR #93)
  then finds it by name. For the first test, fixed addresses on both sides.
- **Testing:** QEMU can't: its DWC2 model (`hw/usb/hcd-dwc2.c`) is host
  only and no QEMU machine has a DWC2 device side. What can be tested on
  the build host: a fake device register file in Rust that plays a host's
  enumeration (reset, GET_DESCRIPTOR, SET_ADDRESS, SET_CONFIGURATION, bulk
  packets) against endpoint 0 and the function (`cargo test`), plus unit
  tests of the descriptors and of NCM's NTB encoding and decoding. The QEMU
  tests keep checking that host mode is unchanged. The real test: a Zero 2
  W cabled to homenodellm (powered by it; `cdc_ether`/`cdc_ncm` on the
  Linux side, tests over SSH without a person), booted from SD or by
  rpiboot over the same cable; a second USB serial adapter for its console;
  one person visit to set it up and try a Mac and a Windows PC.

| Step | What | Size |
|---|---|---|
| 1 | Mode selection from the device tree, `dwc2d` split into host and device start | ~100 lines |
| 2 | Device core: FIFOs, interrupts, endpoints, soft connect | ~600 lines |
| 3 | Endpoint 0, standard requests, descriptors | ~300 lines |
| 4 | Host model with `cargo test` for steps 2–3 | ~300 lines |
| 5 | CDC-ECM function, `network.usb_gadget`, fixed address | ~400 lines |
| 6 | Zero 2 W bench at homenodellm, first Linux test | setup, 1 person visit |
| 7 | Small DHCP server for the computer's side | ~250 lines |
| 8 | CDC-NCM: NTB16 framing, Microsoft OS 2.0 descriptors, tests | ~450 lines |
| 9 | Mac and Windows check | 1 person visit |

About 2400 lines in all (today's `dwc2d` is about 4600), roughly 5–6
sessions: 2 for steps 1–4, 1 for step 5, 1 for steps 7–8, and 1–2 for the
bench and fixes found on the Zero. **Medium**; the risk is the test bench,
not the code.

## Later

- `lsusb` ([#35](https://github.com/redox-rpi/redox-rpi/issues/35), firmware
  session).

Keyboard layouts are in inputd, not dwc2d:
[#36](https://github.com/redox-rpi/redox-rpi/issues/36), branch
[`keyboard-layouts`](https://github.com/redox-rpi/redox-rpi/tree/keyboard-layouts)
(t18 session), Pi test #64 with a person.
