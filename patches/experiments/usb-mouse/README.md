# USB mice and tablets (experiment `usb-mouse`)

Applies after the experiment `usb-hotplug` (`--experiment usb-hotplug
--experiment usb-mouse`).

- **base `0035`:** `dwc2d` runs a USB mouse or tablet (HID) and sends its
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
