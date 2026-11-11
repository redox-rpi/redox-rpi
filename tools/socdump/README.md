# socdump

Reads the Raspberry Pi's clock, PLL, GPIO and display registers and
prints them decoded. It only reads; nothing writes a register. The idea comes from
`pll-inspector` and `pv_dumper` in
[librerpi/rpi-tools](https://github.com/librerpi/rpi-tools), rewritten in
Rust (no code taken). Plan and later steps:
[#224](https://github.com/redox-rpi/redox-rpi/issues/224).

```text
socdump [plls|clocks|gpio|all]   decoded (default all, display included)
socdump display                  HVS, pixel valves and HDMI, decoded
socdump check                    compare the rates with /scheme/rpi-firmware/clocks
socdump regs                     every register as "block address value name", for diff
socdump diff A B                 the registers that differ between two regs dumps
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
- **Display** (`socdump display`): the HVS channels (on/off, size, FIFO
  state, COB range, display list start) and each plane of their display
  lists (format, size, position, scaled or not, raw words); the three
  pixel valves (on/off, timing, pixel clock, refresh rate); the HDMI core
  and HD blocks (timing, sync polarity, scheduler, hotplug, PHY, video
  control, frame count). HDMI is read only when the `hsm` clock and PLLH
  run: reading a block without its clock can hang the bus. Registers whose
  read changes something (the audio FIFO `MAI_DATA`, the CEC data
  registers) are left out.
- **check**: for each clock the firmware reports, our rate next to the
  firmware's, `ok` within 1 %.
- **diff**: two `socdump regs` outputs side by side, only the registers
  that differ or are in one of them only; exit status 0 when equal. For
  closed against open firmware, or one mode against another.

## Where the values come from

| What | From |
|---|---|
| Chip | the devicetree's root `compatible` (`brcm,bcm2837`), else the board revision in `/scheme/rpi-firmware/board` |
| Peripheral address | the devicetree's `/soc` `ranges` (`0x7e000000` -> `0x3f000000` on the 3B+) |
| Crystal | `/clocks/clk-osc` `clock-frequency` (19.2 MHz on the 3B+), else the chip's default |
| Register layout | `src/chip.rs`, one table per chip; offsets from the BCM2835 peripherals manual and Linux `clk-bcm2835.c` |
| Display registers | `src/display.rs`; offsets from Linux `vc4_regs.h` and `vc4_hdmi_regs.h` (BCM2835 table), decoding our own; pixel valve and HVS plane fields checked against librerpi's `pixelvalve.md` and `hvs.md` (no text or code taken) |

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
  clock `emmc2`, peripherals at `0xfe000000`. Its HDMI registers sit at
  other offsets (`HORZA` at 0xe4, `HOTPLUG` at 0x1a8; see `hdmi.md` in
  [librerpi/rpi-open-firmware](https://github.com/librerpi/rpi-open-firmware/tree/master/docs)),
  and it has more PHY registers, so `src/display.rs` needs a second table.
  Not done.
- **BCM2712** (Pi 5): different clock block; most I/O (GPIO) is on the RP1
  chip behind PCIe. Needs its own tables. Not done.

On Linux (Raspberry Pi OS) it builds too and maps `/dev/mem` instead, to
compare with Linux's view.

## Tests

`cargo test` on any host checks the decoding (PLL and divider maths,
devicetree ranges, pixel valve and HDMI timing, display list walk). The
Code quality CI job runs it.

## Display reference dumps (#224 step 4)

- 1920x1080, closed firmware, NanoKVM: Pi test row 337 (no person),
  saved as `reference/pi3bp-firmware-1080p-display.txt` and `.regs`.
- 1280x720: needs `hdmi_mode=4` (with `hdmi_group=1`) in the SD card's
  `config.txt`, since the test Pi reads `config.txt` from the card, not
  from TFTP. So it waits for a card visit.
