# GPIO, I2C and other Pi peripherals

## State

- GPIO, I2C and SPI drivers start at boot (base patches `0045`–`0048`,
  v0.8.0; Pi test #99 passed 21 of 21 without wiring):
  - [#38](https://github.com/redox-rpi/redox-rpi/issues/38) `bcm2835-gpiod`: the 54 pins as `/scheme/gpio`
    (`<pin>/mode`, `value`, `pull`, and the `pins` table); the 3B+ power
    LED as `/scheme/rpi-firmware/power_led`. See
    [gpio](../../patches/experiments/gpio/README.md).
  - [#39](https://github.com/redox-rpi/redox-rpi/issues/39) `bcm2835-i2cd`: I2C bus 1 (pins 3 and 5) as
    `/scheme/i2c`, and the `i2c` tool (`detect`, `dump`, `get`, `set`). See
    [i2c](../../patches/experiments/i2c/README.md).
  - [#42](https://github.com/redox-rpi/redox-rpi/issues/42) `bcm2835-spid`: SPI0 (pins 19, 21, 23, 24, 26) as
    `/scheme/spi`, and the `spi` tool. See
    [spi](../../patches/experiments/spi/README.md).
- [`nowire-check.py`](../../patches/experiments/gpio/nowire-check.py) checks
  all three on a Pi without wiring or a person; each experiment directory
  keeps its `qemu-check.sh`.

## In progress

- Pi tests #38–#40 need a person and wiring: LEDs watched, a DS3231 module
  on pins 1/3/5/6, a jumper from pin 19 to 21. Until then, chip select
  lines, real I2C devices and the LEDs as seen are untested on the Pi.
- The DS3231 as the system clock at boot: branch `ds3231`, with the
  [time](time.md) topic.
- Bluetooth (#54, [firmware](firmware.md)) uses `bcm2835-gpiod` for its
  pins; its branch still applies `gpio` as an experiment and moves on top
  of these defaults after the merge.

## Later

- I2C bus 0, interrupts instead of polling, transfers without a register
  number.
- SPI1 and SPI2 (the auxiliary SPI), SPI with DMA.
- GPIO edge events (interrupts), PWM (also for the 3B+ headphone
  jack), 1-Wire.
