# Experiment: CPU clock and throttle watch

Issue [#20](https://github.com/redox-rpi/redox-rpi/issues/20) (T21), planned for v0.3.0. Extends [`rpi-firmwared`](../../../docs/rpi-firmware.md)
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

Scripts used for the tests on the Pi, run from a clone of this repository
on the build host:

- `clock-timing.py <repo> <pi> [rounds]`: 50 starts at `min`, then at
  `max`, repeated, with temperature and `throttled`.
- `clock-soak.py <repo> <pi> [minutes] [clock]`: start loops without a
  pause for the given time (optionally at a fixed clock), failures,
  temperature and `throttled` after each round of 50.

## Results

- **QEMU** (build `v0.2.0-10-gc489f0c+x097f`, 2026-10-28): the driver
  logs `ARM clock 700 -> 700 MHz` (QEMU has a fixed ARM clock);
  `arm_clock` reads `700000000`; writing `min` and `max` works, `12x`
  fails with "Invalid argument"; the other files stay read-only. The
  throttle watch stays quiet (QEMU does not report throttling).
- **Real Pi 3B+** (test #13, build `v0.3.0-dev-31-g115d7fe+x572d`,
  2026-10-28): the boot log shows `ARM clock 600 -> 1400 MHz`; standard
  checks pass (0 of 300 starts failed); 50 starts take 8.2–8.7 s at 1400 MHz
  against 16.4–17.4 s at 600 MHz. The throttle watch logs the soft
  temperature limit and its end. **But** under continuous load, once the SoC
  reaches about 60 °C, 3–5 % of starts crash (276 of 5100 and 128 of 4000),
  against 5 of 4350 at 600 MHz. Details in
  [test-reports.md](../../../docs/test-reports.md).
- **Real Pi 3B+ with the x18 fix** (test #19, build
  `v0.3.0-dev-59-g1559bca+x9906`, experiments `sigentry-x18` and
  `cpu-clock`, 2026-10-28): standard checks pass, and 30 minutes of start
  loops at 1400 MHz had **0 of 7000** failures, with the soft temperature
  limit reached as often as in #13. The crashes in #13 were the x18 bug from
  #1, not the clock. Ready for v0.3.0 together with the x18 fix.
- Open: the throttle watch logs every on and off of the soft limit (59 times
  in 30 minutes); `arm_clock` shows the requested rate, not the measured
  one (tag `0x30047`).
