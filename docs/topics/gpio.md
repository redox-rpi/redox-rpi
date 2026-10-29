# GPIO, I2C and other Pi peripherals

## State

- Nothing on `main` yet. The firmware sets the pins it needs at boot
  (serial console, SD card); no driver in Redox touches the others.

## In progress

- [#38](https://github.com/redox-rpi/redox-rpi/issues/38) GPIO driver `bcm2835-gpiod` and the 3B+ power LED
  (v0.8.0): experiment [`gpio`](../../patches/experiments/gpio/README.md),
  branch `gpio`.
- [#39](https://github.com/redox-rpi/redox-rpi/issues/39) I2C driver for the BSC controllers (v0.8.0), on top
  of `gpio`, as the base for a DS3231 clock module (with the
  [time](time.md) topic) and sensors.
- [#42](https://github.com/redox-rpi/redox-rpi/issues/42) SPI driver `bcm2835-spid` for SPI0 and the `spi` tool
  (v0.8.0): experiment [`spi`](../../patches/experiments/spi/README.md),
  branch `spi`, on top of `gpio`.

## Later

- SPI1 and SPI2 (the auxiliary SPI), SPI with DMA.
- GPIO edge events (interrupts), PWM (also for the 3B+ headphone
  jack), 1-Wire.
