# USB

## State

- `dwc2d`, a DWC2 USB host driver in base: hubs, split transactions for low-
  and full-speed devices behind the Pi 3B+'s hubs, bulk transfers.
- USB keyboard on the HDMI console of a real Pi 3B+ and Zero 2 W, when
  plugged in before boot.

## In progress

- Branch `dwc2-interrupts`: interrupts instead of polling. Passed on the
  real Pi (test #5, `v0.3.0-dev-52-g7df70f9+xcb30`): `dwc2d` uses about
  5 % of a core instead of a full one; waiting for the merge into `main`.

## Next

- [#17](https://github.com/redox-rpi/redox-rpi/issues/17) hotplug: devices plugged in after boot.

## Later

- [#25](https://github.com/redox-rpi/redox-rpi/issues/25) USB sticks (mass storage), milestone v0.5.0 or v0.6.0.
- Keyboard layouts, `lsusb`
  ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
