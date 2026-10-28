# Experiment: CPU clock follows the load (ondemand governor)

Issue [#23](https://github.com/redox-rpi/redox-rpi/issues/23), planned for
v0.4.0. Builds on base patch `0021` (ARM clock to its maximum, `arm_clock`
file, throttle watch).

```bash
scripts/build-image.sh --experiment cpu-ondemand
```

Base patch `0023` adds a governor thread to `rpi-firmwared`, like Linux's
`ondemand` governor with `raspberrypi-cpufreq`:

- Every 100 ms it reads the per-CPU times from `/scheme/sys/stat` (busy and
  idle milliseconds, kept by the kernel at each context switch) and takes
  the load of the busiest CPU, because all four cores share one clock.
- Above 50 % load it sets the maximum ARM clock (1400 MHz on the 3B+);
  after 2 seconds in a row below 30 % it goes back to the minimum
  (600 MHz). Only these two rates; the firmware picks the voltage.
- The firmware still lowers the clock by itself when the SoC gets hot or
  the voltage drops, as before.
- `/scheme/rpi-firmware/governor` shows the state:

  ```
  mode ondemand
  target min
  load 3
  switches 12
  min 600000000
  max 1400000000
  ```

  Writing `ondemand`, `performance` (always maximum, the behavior of
  `0021`) or `powersave` (always minimum) sets the mode:
  `echo performance > /scheme/rpi-firmware/governor`.
- Writing a rate to `arm_clock` stops the governor (mode `fixed`) until a
  mode is written again.
- The mode is not kept across a reboot: `rpi-firmwared` starts from the
  initfs, before `/etc` is there. A line in an init script can set it.

`rpi-firmwared` leaves its namespace after start (`setrens(0, 0)`), so it
opens `/scheme/sys` before that and opens `stat` inside it for each sample
(the kernel builds the text once per open).

## Test on the Pi

1. Boot. The boot log shows `rpi-firmwared: ARM clock 600 -> 1400 MHz`.
2. `scripts/pi-test.py` standard checks.
3. `ondemand-check.py <repo> <pi>` (in this directory): after a few idle seconds the governor
   shows `target min` and `arm_clock` 600 MHz; 50 starts timed in the modes
   `ondemand`, `performance` and `powersave`. Ondemand should be close to
   performance, and the number of switches grows.
4. Soak: `ondemand-check.py <repo> <pi> 1 30` adds 30 minutes of rounds of
   50 starts with 3 s pauses, so the clock goes up and down each round.
   No failures, no hang, `switches` grows by about two per round.

## Results

- Not tested yet.
