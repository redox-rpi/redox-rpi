# Firmware interface

## State

- `rpi-firmwared` (base patch `0018`) talks to the VideoCore firmware over
  the mailbox and serves `/scheme/rpi-firmware`: model, memory, clocks,
  temperature, throttling. See [rpi-firmware.md](../rpi-firmware.md).
- `reboot` through the watchdog (kernel patch `0002`).

## In progress

- [#20](https://github.com/redox-rpi/redox-rpi/issues/20) (T21) set the ARM clock to 1.4 GHz instead of 600 MHz and
  warn on undervoltage or overheating; branch `cpu-clock`, Pi test #13.
  Builds and works in QEMU (`/scheme/rpi-firmware/arm_clock` reads and
  sets the clock); the real clock change needs the Pi.

## Later

- [#21](https://github.com/redox-rpi/redox-rpi/issues/21) a common DMA plan for the Pi, including the BCM2835 DMA
  controller and the firmware's channel mask (milestone v0.4.0).
- [#7](https://github.com/redox-rpi/redox-rpi/issues/7) power off, v0.5.0 or later (not needed for now).
- Power domains, GPIO expander, DMA channel mask, display, VCHIQ
  ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
