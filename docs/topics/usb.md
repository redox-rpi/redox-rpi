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

- Nothing right now.

## Next

- [#17](https://github.com/redox-rpi/redox-rpi/issues/17) hotplug: devices plugged in after boot.

## Later

- [#25](https://github.com/redox-rpi/redox-rpi/issues/25) USB sticks (mass storage), milestone v0.5.0 or v0.6.0.
- Keyboard layouts, `lsusb`
  ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
