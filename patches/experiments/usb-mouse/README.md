# USB mice and tablets (was experiment `usb-mouse`)

**Default since PR #223:** base `0059` (moved to `patches/base/`) is
in every image: every image with Orbital needs the mouse, and Pi row 220
had none because its image was built without the experiment.
`--experiment usb-mouse` is still accepted and does nothing, so older
commands keep working. This page stays as the patch's documentation.

Issue [#48](https://github.com/redox-rpi/redox-rpi/issues/48). Applies on
main, where hotplug is part of the default patches (`0031`–`0033`):
`--experiment usb-mouse`. Moved over from branch `usb-mouse`, where it was
`0035` on top of the old `usb-hotplug` experiment; renumbered to `0059`
because `0035` is USB serial (PR #70).

- **base `0059`:** `dwc2d` runs a USB mouse or tablet (HID) and sends its
  movements, buttons and wheel to inputd, as it does for the keyboard and
  as `usbhidd` does on xHCI: `MouseRelativeEvent` for a mouse, `MouseEvent`
  with 0..=65535 on both axes for a tablet, `ButtonEvent` (left, right,
  middle) and `ScrollEvent`. Orbital and other inputd consumers read them
  from `/scheme/input/consumer` like any other input.
  - The report descriptor says where X, Y, wheel and buttons are, so mice
    in report protocol (with wheel) and absolute pointers work; a boot
    protocol mouse whose descriptor does not parse uses the boot report.
  - A wireless receiver with keyboard and mouse in one device gets both.
  - Plugged in and out at any time (hotplug rescan); `echo mouse >
    /tmp/dwc2d-cycle` switches its port off and on.
  - While `/tmp/dwc2d-input-log` exists (looked at every second), dwc2d
    prints each mouse event, for tests without a person.
  - Polled every 10 ms on host channel 6 (keyboard 3, USB sticks 4 and 5).

## Tests

- **QEMU** (`scripts/test-usb-mouse.sh`, which needs
  `scripts/qemu-raspi3b.sh --qmp` for absolute events): a `usb-mouse` and a
  `usb-tablet` plugged in after boot, moved, left button, wheel, port
  switched off and on, unplugged; the tablet's middle (16384 of 32767)
  arrives as 32768. `scripts/test-hotplug.sh` still passes (keyboard and
  network).
- QEMU 2026-10-31 on main, `v0.3.0-dev-820-g77ba1ac+x0c8f`:
  `scripts/test-usb-mouse.sh` 18/18, `scripts/test-keyboard.sh` pass,
  `scripts/test-hotplug.sh` 15/15.
- QEMU 2026-11-02 on main with dwc2d `0034` (0059 refreshed for
  `boot_probe`, from the graphics branch `graphics-visit-mouse`),
  `v0.3.0-dev-1035-g39056bf+x0c8f`: mouse test 18/18, keyboard test pass,
  hotplug 15/15.
- Pi test 203 (covers row 56), 2026-11-02, screen visit with Graphics'
  image `graphics-visit-mouse` (this `0059` as in 47859bc): **pass**.
  Serial: `dwc2d: mouse at address 3: relative, 3 button(s), wheel`
  (093a:2510, low speed behind the hub), keyboard at address 5, no panic.
  At the screen the mouse worked well and opened a terminal in Orbital;
  moving windows is slow, which is Orbital's drawing, not the mouse.
- Default patch (PR #223): QEMU 2026-11-06, image without experiments
  (`v0.4.0-dev-569-g738203dc`): mouse test 18/18, keyboard test pass,
  hotplug 15/15. Pi row 255, 2026-11-06, image with only `timesync`
  (`v0.4.0-dev-570-ge437e901+x12cb`): **pass**. `mouse at address 5:
  relative, 3 button(s), wheel` at boot, pi-test passes, the mouse's
  port switched off and on and the mouse found again (its SET_IDLE
  stalls, which is optional and harmless).
