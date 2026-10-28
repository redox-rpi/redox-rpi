# Firmware interface

## State

- `rpi-firmwared` (base patch `0018`) talks to the VideoCore firmware over
  the mailbox and serves `/scheme/rpi-firmware`: model, memory, clocks,
  temperature, throttling. See [rpi-firmware.md](../rpi-firmware.md).
- `reboot` through the watchdog (kernel patch `0002`).

## In progress

- [#20](https://github.com/redox-rpi/redox-rpi/issues/20) (T21) set the ARM clock to 1.4 GHz instead of 600 MHz and
  warn on undervoltage or overheating; branch `cpu-clock`. Pi test #13
  (2026-10-28): 1400 MHz works and starts are twice as fast, the warnings
  work, but under load start crashes go from 0.1 % to 3–5 % once the SoC
  reaches the soft temperature limit (about 60 °C). Stays an experiment;
  next: a fixed lower clock (1200 MHz) below that limit
  ([report](../test-reports.md)).

## Later

- [#23](https://github.com/redox-rpi/redox-rpi/issues/23) the CPU clock follows the load, like Linux `ondemand`:
  600 MHz when idle, 1400 MHz under load (milestone v0.4.0).
- [#21](https://github.com/redox-rpi/redox-rpi/issues/21) a common DMA plan for the Pi, including the BCM2835 DMA
  controller and the firmware's channel mask (milestone v0.4.0).
- [#7](https://github.com/redox-rpi/redox-rpi/issues/7) power off, v0.5.0 or later (not needed for now).
- Power domains, GPIO expander, DMA channel mask, display, VCHIQ
  ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
