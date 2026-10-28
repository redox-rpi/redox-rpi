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
