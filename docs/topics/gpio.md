# GPIO, I2C and other Pi peripherals

## State

- Nothing on `main` yet. The firmware sets the pins it needs at boot
  (serial console, SD card); no driver in Redox touches the others.

## In progress

- [#38](https://github.com/redox-rpi/redox-rpi/issues/38) GPIO driver `bcm2835-gpiod` and the 3B+ power LED
  (v0.8.0): experiment [`gpio`](../../patches/experiments/gpio/README.md),
  branch `gpio`.
- [#39](https://github.com/redox-rpi/redox-rpi/issues/39) I2C driver `bcm2835-i2cd` for bus 1 and the `i2c`
  tool (v0.8.0): experiment [`i2c`](../../patches/experiments/i2c/README.md),
  branch `i2c`, on top of `gpio`. Base for a DS3231 clock module (with the
  [time](time.md) topic) and sensors.
- [#42](https://github.com/redox-rpi/redox-rpi/issues/42) SPI driver `bcm2835-spid` for SPI0 and the `spi` tool
  (v0.8.0): experiment [`spi`](../../patches/experiments/spi/README.md),
  branch `spi`, on top of `i2c` (gpio, i2c and spi go in together).
- Branch `spi` is on current main (2026-10-30, image
  `v0.3.0-dev-393-ga24b8b8+xcd36`): all three QEMU checks pass. Pi test
  #99 checks what needs no person or wiring
  ([`nowire-check.py`](../../patches/experiments/gpio/nowire-check.py):
  output read-back, pulls, `power_led`, I2C errors, SPI with the MISO
  pull). Pi tests #38–#40 (LEDs seen by a person, DS3231, jumper wires)
  wait for a visit.
- Before the PR: gpio patch `0041` (`power_led`) must be regenerated on top
  of the ondemand governor (base `0040`, PR #76), which touches the same
  lines of `rpi-firmwared`; the patch numbers then follow the free ones.

## Later

- I2C bus 0, interrupts instead of polling, transfers without a register
  number.
- SPI1 and SPI2 (the auxiliary SPI), SPI with DMA.
- GPIO edge events (interrupts), PWM (also for the 3B+ headphone
  jack), 1-Wire.
