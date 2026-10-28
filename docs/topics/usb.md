# USB

## State

- `dwc2d`, a DWC2 USB host driver in base: hubs, split transactions for low-
  and full-speed devices behind the Pi 3B+'s hubs, bulk transfers.
- USB keyboard on the HDMI console of a real Pi 3B+ and Zero 2 W, when
  plugged in before boot.

## In progress

- Branch `dwc2-interrupts`: interrupts instead of polling (CPU time in QEMU
  1.5 s instead of 4 min 21 s); Pi test #5 queued.

## Next

- [#17](https://github.com/redox-rpi/redox-rpi/issues/17) hotplug: devices plugged in after boot.

## Later

- [#25](https://github.com/redox-rpi/redox-rpi/issues/25) USB sticks (mass storage), milestone v0.5.0 or v0.6.0.
- Keyboard layouts, `lsusb`
  ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
