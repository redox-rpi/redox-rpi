# Experiment: I2C

Issue [#39](https://github.com/redox-rpi/redox-rpi/issues/39), planned for
v0.8.0. Builds on the [gpio](../gpio/README.md) experiment (#38), so both
go in:

**Default since PR (v0.8.0):** base patch `0047` (was `0043` here), the
`i2c` tool is in every image. The name `i2c` is still accepted and does
nothing; `qemu-check.sh` stays here. Pi test #99 passed (no wiring); #39
(DS3231 module) waits for a visit.

- **Base patch, `bcm2835-i2cd`** (new driver in the initfs, after
  `bcm2835-gpiod` and `rpi-firmwared`): I2C bus 1 (BSC1, header pin 3 SDA
  and pin 5 SCL) as the scheme `i2c`:

  | Path | Read | Write |
  |---|---|---|
  | `/scheme/i2c/1/scan` | the addresses that answer, one per line (hex) | |
  | `/scheme/i2c/1/<addr>` (`03`–`77`) | from register N on, N = file offset | the same |

  A read or write at offset N sends the register number N, then reads or
  writes the bytes (with a stop between, which the DS3231 and most sensors
  accept). A missing device gives "No such device or address". At start
  the driver sets GPIO 2 and 3 to ALT0 through `/scheme/gpio` and the clock
  to 100 kHz from the core clock that `rpi-firmwared` reports. Polled, no
  interrupts; bus 0 (the HAT EEPROM pins) is not served.
- **Tool `i2c`** (`tools/i2c`, package from this repository), like
  Linux's i2c-tools, all numbers hex:

  ```
  i2c detect                      table of the addresses that answer
  i2c dump 68                     all 256 registers
  i2c get 68 0 7                  7 registers from 00 on
  i2c set 68 7 12                 write 12 to register 07
  ```
- **`scripts/qemu-raspi3b.sh --device <spec>`** adds a QEMU device, e.g.
  the clock on bus 1.

Patch numbers here are placeholders; the merge gives the real ones. The
DS3231 as the system clock (reading the time at boot) is the next step,
agreed with the time topic.

## QEMU

[`qemu-check.sh`](qemu-check.sh): QEMU emulates the BSC controllers and has
a DS1338 clock (same time registers as the DS3231, plus RAM at 08–3f),
attached at 0x68 on bus 1. The check scans the bus, reads the year, writes
and reads back RAM, and checks the errors for a missing device and a bad
address.

## Test on the Pi

Needs a person to wire a DS3231 module (with its battery), power off:

| DS3231 | Pi header |
|---|---|
| VCC | pin 1 (3.3 V, not 5 V) |
| SDA | pin 3 (GPIO 2) |
| SCL | pin 5 (GPIO 3) |
| GND | pin 6 |

The small modules made for the Pi plug straight onto pins 1 to 7 (check
that their GND lands on pin 6, VCC on pin 1).

1. Boot, `scripts/pi-test.py` standard checks; the boot log shows
   `bcm2835-i2cd: bus 1 at 0x3f804000, 100 kHz`.
2. `cat /scheme/gpio/2/mode /scheme/gpio/3/mode`: `alt0` twice.
3. `i2c detect`: 68 (and 57 on modules with an AT24C32 EEPROM).
4. `i2c get 68 0 7`: seconds, minutes, hours, weekday, date, month, year in
   BCD; run it twice a few seconds apart, the seconds go on.
5. `i2c get 68 11 2`: temperature, first byte in °C.
6. `i2c set 68 7 12`, `i2c get 68 7` gives `12`, then `i2c set 68 7 0`
   (alarm 1 seconds register, harmless with the alarm off).
7. `i2c get 50 0`: "No such device or address" (unless something is at
   0x50).
8. Without the module: `i2c detect` shows an empty table, no hang.

## Results

- QEMU, image `v0.3.0-dev-146-g5a7fc2c+x630f` (gpio and i2c together):
  all 9 checks of `qemu-check.sh` pass: `bus 1 at 0x3f804000, 100 kHz
  (core 350 MHz)`, the scan finds only 68, the year reads `26`, RAM
  written at 08 reads back, 0x50 gives "No such device or address". The
  DS1338 wraps at 0x40, so `i2c dump 68` repeats its registers there.
- Pi test: queued as #39 (needs the DS3231 wired).
