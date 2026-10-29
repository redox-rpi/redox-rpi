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

## Later

- GPIO edge events (interrupts), SPI, PWM (also for the 3B+ headphone
  jack), 1-Wire.
