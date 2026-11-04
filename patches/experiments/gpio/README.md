# Experiment: GPIO

Issue [#38](https://github.com/redox-rpi/redox-rpi/issues/38), planned for
v0.8.0.

**Default since PR (v0.8.0):** base patches `0045` (`bcm2835-gpiod`) and
`0046` (`power_led`); they were `0040`–`0042` here while this was an
experiment (the spin fix `0042` is part of `0045` now). The name `gpio` is
still accepted by `build-image.sh` and does nothing; `qemu-check.sh` and
`nowire-check.py` stay here. Pi test #99 passed (no wiring); #38 (LEDs
watched by a person) waits for a visit.

- **Base patch 1, `bcm2835-gpiod`** (new driver in the initfs, after
  `rpi-firmwared`): the 54 GPIO pins of the BCM2835/BCM2837 as the scheme
  `gpio`:

  | Path | Read | Write |
  |---|---|---|
  | `/scheme/gpio/pins` | one line per pin: `pin mode level` | |
  | `/scheme/gpio/<pin>/mode` | `in`, `out`, `alt0`–`alt5` | the same |
  | `/scheme/gpio/<pin>/value` | `0` or `1`, the level on the pin | `0` or `1`: the output level (on the pin in mode `out`) |
  | `/scheme/gpio/<pin>/pull` | the last setting, or `unknown` | `up`, `down`, `none` |

  ```
  echo out > /scheme/gpio/29/mode
  echo 1 > /scheme/gpio/29/value
  ```

  The pins of the serial console (14, 15) and the SD card (48–53) are
  refused ("Operation not permitted"): a wrong mode would cut the console
  or the root file system. No edge events or interrupts yet. The pull
  resistors cannot be read back from the hardware.
- **Base patch 2, `rpi-firmwared`:** `/scheme/rpi-firmware/power_led`,
  `on` or `off`. On the Pi 3B+ the red power LED hangs on the firmware's
  GPIO expander (pin 2, active low, as in Linux's devicetree), so only the
  firmware can switch it (property tags `0x30041` and `0x38041`). Other
  boards do not have the file.
- **Activity LED:** the green LED is GPIO 29 on the Pi 3B+ (on when high)
  and on the Zero 2 W (on when low), so `bcm2835-gpiod` switches it.

Patch numbers here are placeholders; the merge gives the real ones.

## QEMU

[`qemu-check.sh`](qemu-check.sh): QEMU emulates the GPIO block, so modes
and output levels can be checked. Pulls have no effect there, and QEMU is
a Pi 3B, which has no `power_led` file.

## Test on the Pi

Needs a person to look at the LEDs; the jumper wire step is optional.

1. Boot, `scripts/pi-test.py` standard checks.
2. Green LED: `echo out > /scheme/gpio/29/mode`, then `echo 1 >
   /scheme/gpio/29/value` and `echo 0 > ...`: the LED goes on and off.
3. Red LED: `echo off > /scheme/rpi-firmware/power_led`, `cat` it, then
   `on`.
4. Optional, with a jumper wire between pin 11 (GPIO 17) and pin 13
   (GPIO 27): GPIO 17 `out`, GPIO 27 `in`; writing 1 and 0 to 17 shows in
   `/scheme/gpio/27/value`. Without the wire: GPIO 27 `in` with `pull` `up`
   reads 1, with `down` reads 0.

## Results

- QEMU, image `v0.3.0-dev-146-g5a7fc2c+x630f` (gpio and i2c together):
  all 10 checks of `qemu-check.sh` pass. The first run hung on writing a
  pull: a 10 µs `nanosleep` in the driver did not come back; base patch 3
  spins instead.
- Pi test: queued as #38.
