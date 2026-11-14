# USB serial adapters (was experiment `usb-serial`)

Issue: [#50](https://github.com/redox-rpi/redox-rpi/issues/50), v0.8.0,
topic:usb. **Default once this branch is merged:** the patch is base
`0035` in `patches/base/`, after the hotplug patches `0031`–`0033`.
`--experiment usb-serial` is still accepted and adds nothing. Below,
`0036` is the number it had as an experiment.

- **base `0036`:** `dwc2d` runs USB serial adapters, the chips in Zigbee
  and Thread coordinator sticks (SONOFF ZBDongle-P/-E, SkyConnect) and
  many Arduino boards:
  - CDC-ACM (any device with a communications interface 2/2 and a data
    interface), FTDI (0403:6001/6010/6011/6014/6015), CP210x
    (10c4:ea60/ea61/ea63/ea70/ea71/ea80) and CH340/CH341
    (1a86:7523/5523/7522).
  - Each is `/scheme/usb-serial/<ports>`, named by its hub ports from the
    root like USB sticks (e.g. `1.3`); `ls /scheme/usb-serial` lists them.
    Read and write bytes; reads block until data comes (or EAGAIN with
    O_NONBLOCK) and raise read events.
  - Speed: `/scheme/usb-serial/<ports>/baud` reads and takes a number;
    `tcgetattr`/`tcsetattr` work as on a pty (speed, data bits, parity
    and stop bits from `c_cflag`). Ports start at 115200 8N1; DTR and RTS
    go up when a port is opened.
  - Plugged in and out at any time (hotplug rescan); handles of an
    unplugged adapter return ENODEV. `echo serial > /tmp/dwc2d-cycle`
    switches its port off and on.
  - While `/tmp/dwc2d-serial-log` exists (looked at every second), dwc2d
    prints what is read and written.
  - The open port is read every 5 ms on host channel 7 (keyboard 3, USB
    sticks 4 and 5, mouse 6); a full-speed adapter behind the Pi 3B+'s
    high-speed hub is read packet by packet through the hub's transaction
    translator. One port is read at a time.

## Tests

- **QEMU** (`scripts/test-usb-serial.sh`), **15 of 15 pass**: QEMU's
  `usb-serial` (an FTDI FT232) plugged in after boot with its other end on
  a Unix socket; listed, write and read, speed set to 9600 (a bad value
  is refused), port switched off and on, unplugged and plugged in again.
  QEMU's DWC2 model goes on retrying a halted channel's NAKed packet and
  aborts once the device is gone; dwc2d replaces that packet with a SETUP
  when a port is closed (`Dwc2::abandon`, as `halt_device` does on
  removal), which is harmless on the Pi. CDC-ACM, CP210x and CH340 have
  no QEMU device; they share the scheme and the transfer code and differ
  only in their setup requests, checked on the Pi.
- **Pi**: test #62 in docs/test-plan.md, with a person and a stick.

## Merge to main

Draft pull request on top of `usb-hotplug`, with main merged in
(2026-10-30). The patch applies unchanged (same patch ID). **It waits
for Pi test #62, which needs a person.**

- **QEMU after merging main** (`v0.3.0-dev-332-gbf1ec19+x41ea`):
  `scripts/test-usb-serial.sh` 15/15 and `scripts/test-hotplug.sh` 15/15.
- **Proven on the Pi:** nothing of its own yet. The hotplug and channel
  code it builds on passed #35 and #67.
- **Still needs a person (#62):** plug a CP210x or CH340 stick (and FTDI or
  CDC-ACM if at hand) into the 3B+. Then read and write through a
  loopback jumper or the stick's own answers, set the speed, unplug and
  plug in again. QEMU has only an FTDI model, so CDC-ACM, CP210x and CH340
  are proven only on the Pi.
