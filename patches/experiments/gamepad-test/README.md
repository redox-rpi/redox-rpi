# Experiment: gamepad-test, a window that shows a gamepad live

For [#320](https://github.com/redox-rpi/redox-rpi/issues/320): puts
[`tools/gamepad-test`](../../../tools/gamepad-test) into the image, with
its launcher entry ("Gamepad test"). Build it after the desktop experiments:

```bash
scripts/build-image.sh --experiment timesync --experiment rpi-fb --experiment orbital \
    --experiment orbital-apps --experiment cosmic-apps-source --experiment desktop \
    --experiment gamepad-test
```

The window shows Orbital's `ControllerButton` and `ControllerAxis` events
while it is the front window:

- the 15 buttons of SDL's "Redox OS Controller" (A B X Y LB RB BACK START
  LS RS GUIDE and the D-pad UP DOWN RIGHT LEFT) as boxes that light up
  while pressed;
- the sticks X/Y and RX/RY as a cross with a dot (up is a positive Y, as
  dwc2d sends it), the triggers Z and RZ (0..255) as bars, each with its
  value;
- the controller id, the number of events, the last event as text, and how
  many events had a code it does not know.

Every event also goes to stdout (`gamepad-test: controller 0: button A
pressed`). Esc or closing the window quits.

USB pads send these events only with dwc2d from PR
[#378](https://github.com/redox-rpi/redox-rpi/pull/378) (branch
`usb-gamepad`); before it, a pad is a mouse. The app itself does not need
#378: anything that sends controller events through inputd works.
`gamepad-test --send` (as root, while Orbital runs) sends a fixed sequence
of all buttons and axes through inputd, as dwc2d would. This is how QEMU,
which has no USB gamepad, tests the window.

## Installing it later, on a running system

The package `gamepad-test` holds the binary, the launcher entry and the
icon. It is a static binary, so it needs no other package.

- On an image with `pkg-hold` and `pkg-repo`, it comes from our
  repository (`scripts/publish-pkg-repo.sh gamepad-test` on the build
  host): `sudo pkg install gamepad-test`. On network boot and direct boot
  the root filesystem is in RAM, so the package is gone after a reset.
- With `pkg-persist` and an SD card with a data partition: `sudo
  pkg-persist install gamepad-test`. This keeps it across resets.
- The launcher reads its entries when it starts, so the new entry shows
  only after the next login. Until then, start it from a terminal:
  `gamepad-test`.
