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

## On the real Pi 3B+ (Pi test #261, 2026-11-06)

Closed firmware, network image `v0.4.0-dev-587-g9bf64d7`, NanoKVM on HDMI.
The full output is in [`reference/`](reference/) (decoded `.txt`, raw
`.regs` for `diff`, the firmware's own rates in `.mailbox`).

- **Matches the firmware exactly:** ARM 1400 MHz (`pllb` 2800 / 2), EMMC
  200 MHz (`pllc_per` 1200 / 6), UART 48 MHz (`plld_per` 500 / 10.4167).
- **PLLs:** A 2400, B 2800, C 2400, D 2000, H 1485 MHz, all locked.
- **HDMI is visible already:** `pllh_pix` = 1485 / 10 = **148.5 MHz**, the
  1080p60 pixel clock; `hsm` (HDMI state machine) 163.68 MHz from
  `plld_per`. That's the start of step 2 of #224.
- **Firmware-driven clocks:** `vpu`, `h264`, `isp` and `sdram` keep their
  reset values in the clock manager (`0x245`/`0x244`/`0x4006`, divider 3,
  the same as in QEMU) while the firmware reports 250, 250, 250 and 400 MHz.
  The firmware drives them some other way, so `socdump` shows them as `fw`
  and `check` skips them.
- **v3d:** the register says 300 MHz (the firmware's maximum), while the
  firmware reports 250 MHz. Still open: is the mailbox value stale, or
  does v3d switch to 250 MHz some other way?
- **GPIO:** 2/3, 7–11, 14/15, 40–43, 48–53 `alt0`; 29 and 47 outputs.

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
