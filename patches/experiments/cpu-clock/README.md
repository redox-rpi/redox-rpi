# Experiment: CPU clock and throttle watch

Task [T21](../../../docs/tasks.md), planned for v0.3.0. Extends [`rpi-firmwared`](../../../docs/rpi-firmware.md)
(base patch `0018`):

- **ARM clock to its maximum at start.** The firmware starts the ARM cores
  at 600 MHz; the Pi 3B+ runs at up to 1.4 GHz. The driver asks the
  firmware for the maximum and sets it (property tag `0x38002`), as Linux's
  `raspberrypi-cpufreq` with the performance governor would. The firmware
  raises the core voltage itself and lowers the clock again when the SoC
  gets hot (soft limit 60 °C on the 3B+, hard limit 85 °C) or the supply
  voltage drops.
- **`/scheme/rpi-firmware/arm_clock`:** the ARM clock in Hz. Writing a rate
  in Hz, `max` or `min` sets it, for comparisons in one boot:
  `echo min > /scheme/rpi-firmware/arm_clock`.
- **Throttle watch:** a thread reads the throttling flags every 5 seconds
  and logs each change, like Linux's `raspberrypi-hwmon`:
  `rpi-firmwared: warning: undervoltage (flags 0x00050005, 61.2 °C, ARM 600 MHz)`.

```bash
scripts/build-image.sh --experiment cpu-clock
```

## Test on the Pi

1. Boot. The boot log shows `rpi-firmwared: ARM clock 600 -> 1400 MHz`.
2. `scripts/pi-test.py` standard checks.
3. Speed: `time` the start loop at both clocks in the same boot
   (`echo min > /scheme/rpi-firmware/arm_clock`, then `max`), and note
   the temperature and `throttled` after a few minutes at full clock.
4. Leave it running for 30 minutes: no new errors, `throttled` without
   bit 0 (undervoltage); a warning in the log if a bit was set.

## Results

None yet.
