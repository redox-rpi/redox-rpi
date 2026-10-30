# Experiment: CPU clock follows the load (ondemand governor)

Issue [#23](https://github.com/redox-rpi/redox-rpi/issues/23), planned for
v0.4.0. Builds on base patch `0021` (ARM clock to its maximum, `arm_clock`
file, throttle watch).

```bash
scripts/build-image.sh --experiment cpu-ondemand
```

Base patch `0050` adds a governor thread to `rpi-firmwared`, like Linux's
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

Kernel patch `0050` fixes the CPU times in `sys:stat`: each context switch
added its time in whole milliseconds and dropped the rest, so a CPU that
switches every few microseconds (many system calls, such as the start loop)
looked almost idle. The kernel now keeps nanoseconds and still shows
milliseconds.

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

- **QEMU** (build `v0.3.0-dev-92-g663bbe4+xc99c`, without kernel `0050`,
  2026-10-28): the `governor` file reads as above; writing `performance`,
  `powersave` and `ondemand` works and counts switches, `bogus` fails with
  "Invalid argument", writing `arm_clock` sets mode `fixed`. QEMU has a
  fixed 700 MHz clock (min = max). The load stays at 100 % even when the
  shell is idle: in QEMU `dwc2d` and `bcm2835-sdhcid` poll without pause
  (40 s and 47 s of CPU time after a minute), so ondemand rightly stays at
  `max` there. Only the real Pi can show the switch to `min`. The same run
  showed the millisecond rounding (one CPU with 220000 switches in 5 s
  counted 0.6 s), hence kernel `0050`.
- **QEMU with kernel `0050`** (build `v0.3.0-dev-93-g2d1a2e4+xc99c`,
  2026-10-28): the CPU that runs the polling `bcm2835-sdhcid` now shows
  4124 ms busy in a 4.1 s sample (100 %), the others 0–25 %. The governor
  stays at `max` in ondemand, as expected while a driver polls;
  `powersave` gives `target min`. Ready for the Pi test.
