# Pi Zero 2 W

## State

- The Pi 3B+ SD card image of v0.2.0 boots on a Zero 2 W: USB keyboard
  plugged in before boot, network over the HAT's RTL8152, time over NTP.
- No image of its own is needed: the same image runs on both boards
  ([#8](https://github.com/redox-rpi/redox-rpi/issues/8), closed). For network boot,
  `scripts/netboot.sh --board zero2w` uses a smaller image for the 512 MB RAM.

## Next

- [#19](https://github.com/redox-rpi/redox-rpi/issues/19) fast network boot: U-Boot's r8152 receive buffer is too small.
- [#17](https://github.com/redox-rpi/redox-rpi/issues/17) USB hotplug, the same as on the Pi 3B+.

## Later

Proposals, not planned for v0.3.0 or v0.4.0. They are ordered by benefit for
the Zero. Each one gets an issue when someone takes it on.

1. **Network over the USB cable (gadget mode).** The Zero's only USB port
   works as a device instead of a host, and the Zero shows up as a network
   adapter on the computer it is plugged into. Then it needs no HAT. It
   needs device mode in `dwc2d`, which today is host only. The port is
   either host (HAT, keyboard) or device, not both. Medium.
2. **A lean system for 512 MB.** Measure the free memory after boot and
   which services use it. Leave out what a server does not need. This also
   allows a larger image for network boot than the 128 MiB of today. Small.
3. **Clock and heat on the Zero.** Test the CPU clock work
   ([#20](https://github.com/redox-rpi/redox-rpi/issues/20)) on the Zero too:
   its board is smaller, it gets hot sooner and throttles earlier than the
   Pi 3B+. Small, once #20 is on `main`.
4. **Wi-Fi**, [#26](https://github.com/redox-rpi/redox-rpi/issues/26) (milestone v0.6.0) (CYW43436 over SDIO, firmware blob, WPA2). Once sdhost reads
   the SD card ([#6](https://github.com/redox-rpi/redox-rpi/issues/6)), the
   second SD controller (the one `bcm2835-sdhcid` drives today) is free for
   the Wi-Fi chip, as on Linux. It helps the Pi 3B+ too. Large.
