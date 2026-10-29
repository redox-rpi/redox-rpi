# Experiment: SPI

Issue [#42](https://github.com/redox-rpi/redox-rpi/issues/42), planned for
v0.8.0. Builds on the [gpio](../gpio/README.md) experiment (#38), so both
go in:

```bash
scripts/build-image.sh --experiment gpio --experiment spi
```

Not together with the `i2c` experiment yet: both patches add their driver
to the same lines (Makefile, workspace, drivers target). The merge puts
them side by side.

- **Base patch, `bcm2835-spid`** (new driver in the initfs, after
  `bcm2835-gpiod` and `rpi-firmwared`): SPI0 (header pins 19 MOSI, 21
  MISO, 23 SCLK, 24 CE0, 26 CE1) as the scheme `spi`:

  | Path | Read | Write |
  |---|---|---|
  | `/scheme/spi/0/cs0`, `cs1` | the bytes clocked in during the last write on this handle (once) | one full-duplex transfer, chip select held for all bytes (at most 4096) |
  | `/scheme/spi/0/speed` | SCLK in Hz | Hz; the nearest even divider of the core clock at or below |
  | `/scheme/spi/0/mode` | `0`–`3` | the same (CPOL, CPHA) |

  At start the driver sets GPIO 7 to 11 to ALT0 through `/scheme/gpio`
  and the bus to 1 MHz, mode 0. Polled, no DMA or interrupts; chip select
  active low.
- **Tool `spi`** (`tools/spi`), bytes in hex:

  ```
  spi 9f 0 0 0                    one transfer, prints the answer
  spi -s 500000 -m 3              set speed and mode
  spi -l 4096                     loopback check (MOSI wired to MISO)
  spi -c 1 ...                    chip select 1 instead of 0
  spi info                        speed and mode
  ```

Patch numbers here are placeholders; the merge gives the real ones.

## QEMU

[`qemu-check.sh`](qemu-check.sh): QEMU emulates SPI0, with a W25Q32 flash
on the bus (`scripts/qemu-raspi3b.sh --device w25q32,bus=spi`; the same
`--device` change as on the `i2c` branch). QEMU does not drive the
chip select lines, so the flash stays selected and only fixed-length
commands work: the check reads its JEDEC ID twice and its status register,
sets speed and mode, and runs a 4096-byte transfer to see it end.

## Test on the Pi

Needs a person to put one jumper wire, power off: **pin 19 (MOSI) to pin
21 (MISO)**.

1. Boot, `scripts/pi-test.py` standard checks; the boot log shows
   `bcm2835-spid: bus 0 at 0x3f204000, 1000 kHz`.
2. `cat /scheme/gpio/9/mode /scheme/gpio/10/mode /scheme/gpio/11/mode`:
   `alt0` three times.
3. `spi 12 34 56`: answers `12 34 56`.
4. `spi -l 4096`: `0 wrong`; then the same at `-s 10000000`, `-s 100000`
   and with `-m 3`.
5. `spi -c 1 -l 64`: `0 wrong` (MISO does not depend on the chip select).
6. Without the wire (or before putting it): `spi 12 34` answers `00 00`
   or `ff ff`, and `spi -l 64` reports wrong bytes, no hang.

## Results

- QEMU, image `v0.3.0-dev-145-gf08c7c0+x7b09` (gpio and spi): all checks
  of `qemu-check.sh` pass: `bus 0 at 0x3f204000, 1000 kHz (core 350
  MHz)`, GPIO 7 to 11 in alt0, JEDEC ID `00 ef 40 16 00 00 00` twice,
  status `00`, speed 500000 and mode 3 set, mode 4 refused, a 4096-byte
  transfer ends (the flash answers garbage to it, as expected). The
  bus-directory pattern was fixed after the run and checked against its
  log.
- Pi test: queued as #40.
