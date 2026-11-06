# socdump

Reads the Raspberry Pi's clock, PLL and GPIO registers and prints them
decoded. It only reads; nothing writes a register. The idea comes from
`pll-inspector` and `pv_dumper` in
[librerpi/rpi-tools](https://github.com/librerpi/rpi-tools), rewritten in
Rust (no code taken). Plan and later steps:
[#224](https://github.com/redox-rpi/redox-rpi/issues/224).

```text
socdump [plls|clocks|gpio|all]   decoded (default all)
socdump check                    compare the rates with /scheme/rpi-firmware/clocks
socdump regs                     every register as "block address value name", for diff
```

It needs root (it maps `/scheme/memory/physical@dev`). It is part of the
SD card image (`config/raspi3bp-sdcard.toml`).

## What it shows

- **PLLs** A, B, C, D and H: on/off, locked, the multiplier (`ndiv` +
  `frac`/2^20, doubled by the feedback prediv), `pdiv`, the rate, and each
  channel's divider and rate (`pllb_arm`, `pllc_core0`, `plld_per`,
  `pllh_pix`, ...).
- **Clocks** of the clock manager: source, enabled, divider and rate
  (`vpu`, `emmc`, `uart`, `pwm`, `gp0`-`gp2`, `dpi`, `hsm`, ...).
- **GPIO**: the function of each pin (`in`, `out`, `alt0`-`alt5`) and its
  level.
- **check**: for each clock the firmware reports, our rate next to the
  firmware's, `ok` within 1 %.

## Where the values come from

| What | From |
|---|---|
| Chip | the devicetree's root `compatible` (`brcm,bcm2837`), else the board revision in `/scheme/rpi-firmware/board` |
| Peripheral address | the devicetree's `/soc` `ranges` (`0x7e000000` -> `0x3f000000` on the 3B+) |
| Crystal | `/clocks/clk-osc` `clock-frequency` (19.2 MHz on the 3B+), else the chip's default |
| Register layout | `src/chip.rs`, one table per chip; offsets from the BCM2835 peripherals manual and Linux `clk-bcm2835.c` |

## Other Pi models

The tables in `src/chip.rs` are per SoC, and addresses are bus addresses
turned into CPU addresses through the devicetree, so a new board is a new
table, not a rewrite:

- **BCM2837** (Pi 3B, 3B+, Zero 2 W): done.
- **BCM2711** (Pi 4): crystal 54 MHz, no PLLH, no feedback prediv, extra
  clock `emmc2`, peripherals at `0xfe000000`. Not done.
- **BCM2712** (Pi 5): different clock block; most I/O (GPIO) is on the RP1
  chip behind PCIe. Needs its own tables. Not done.

On Linux (Raspberry Pi OS) it builds too and maps `/dev/mem` instead, to
compare with Linux's view.

## Tests

`cargo test` on any host checks the decoding (PLL and divider maths,
devicetree ranges).
