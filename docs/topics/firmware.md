# Firmware interface

## State

- `rpi-firmwared` (base patch `0018`) talks to the VideoCore firmware over
  the mailbox and serves `/scheme/rpi-firmware`: model, memory, clocks,
  temperature, throttling. See [rpi-firmware.md](../rpi-firmware.md).
- The ARM clock runs at its maximum (1400 MHz on the 3B+, twice as fast as
  600 MHz), and undervoltage or overheating is logged (base patch `0021`,
  [#20](https://github.com/redox-rpi/redox-rpi/issues/20), Pi tests #13 and
  #19: 0 of 7000 starts failed at 1400 MHz with the x18 fix).
- `reboot` through the watchdog (kernel patch `0002`).

## In progress

- [#7](https://github.com/redox-rpi/redox-rpi/issues/7) power off (v0.5.0): experiment
  [`poweroff`](../../patches/experiments/poweroff/README.md), kernel patch
  `0007` and a `poweroff` tool, branch `poweroff`. Waits for a Pi test with
  a person at the Pi.

## Later

- Throttle watch: fewer lines for the soft temperature limit (59 in 30
  minutes under load); show the measured ARM clock (tag `0x30047`) next to
  the requested one.
- [#23](https://github.com/redox-rpi/redox-rpi/issues/23) the CPU clock follows the load, like Linux `ondemand`:
  600 MHz when idle, 1400 MHz under load (milestone v0.4.0).
- [#21](https://github.com/redox-rpi/redox-rpi/issues/21) a common DMA plan for the Pi, including the BCM2835 DMA
  controller and the firmware's channel mask (milestone v0.4.0).
- Power domains, GPIO expander, DMA channel mask, display, VCHIQ
  ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
