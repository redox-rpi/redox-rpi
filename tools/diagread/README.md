# diagread

Reads the display sampler of our open firmware
([rpi-open-firmware #66](https://github.com/redox-rpi/rpi-open-firmware/issues/66))
and finds display stalls, such as the 2 s hangs of the graphics driver
on the Zero 2 W ([#563](https://github.com/redox-rpi/redox-rpi/issues/563)).
It works on Redox and on Linux, with no interface that only Redox has.

```text
diagread status            header: period, records, dropped, marker
diagread dump [N]          the last N records (default 20), decoded
diagread stalls [MS]       stalls of MS or more (default 50) in the ring
diagread watch [MS]        the same, live, until Ctrl-C
diagread axi [S]           AXI bus counts per second, last S seconds (10)
diagread save FILE         the whole ring as a file
diagread marker VALUE      write the marker word
diagread start US | stop   Linux only: start or stop by mailbox (/dev/vcio)

options: --addr 0xADDR (ring address, if the devicetree has none)
         --file FILE   (a saved ring, on any host)
```

It needs root (it maps `/scheme/memory/physical@dev` on Redox and
`/dev/mem` on Linux). It is part of both Pi 3 images.

## How it works

While the ARM runs, the VPU firmware samples the display hardware every
period and writes one record into a 1 MB ring in the VideoCore's part of
the memory, which no ARM kernel uses: 2 MB into it, below the
framebuffer, so ARM 0x3e200000 on a 1 GB board (rof #67 and later; not
at the very end, which the ARM sees as the peripherals). The firmware puts the ring's
address into the devicetree: `/chosen redox-rpi,diag-ring = <address
bytes>`. The ring holds about 32,000 records, 32 s at 1 ms.

Start it with a line in `config.txt` (our firmware only; Broadcom's
ignores it):

```text
diag_period=1000
```

On Linux, `diagread start 1000` does the same at run time through the
mailbox (tag `0x000f0001`).

Each record holds the VPU timer, HVS `DISPSTAT` and `DISPSTAT1` (channel
1 and its frame count), pixel valve 2 status, the HDMI frame count, the
thermal sensor (the firmware starts it as Linux does), the ARM clock word
(PLLB and the ARM divider) and the marker. Firmware before the Test 530
fixes reads a fixed 268 °C and shows 600 MHz as 1200.

## What `stalls` and `watch` report

| Event | Meaning |
|---|---|
| `stall hdmi frames` | the HDMI frame count stood still: the display stopped scanning out |
| `stall hvs1 frames` | the HVS channel 1 frame count stood still |
| `stall marker` | the marker stood still, once it moved at all (a driver that writes the marker on each flip: no flips) |
| `gap` | the sampler itself had no record for MS or more (and more than 3 periods): the VPU was busy |
| `flag` | an HVS error bit went from 0 to 1 (underflow, line or frame error, COB low, DMA error); they stay set until written back |
| `clock` | the ARM clock changed (governor or throttling) |

Frame counters move once per frame, so `stall hdmi` and `stall hvs1`
need at least 40 ms (two frames at 50 Hz), even with a smaller MS.

The firmware keeps writing while diagread copies the ring. Records in the
oldest slots that were overwritten during the copy are left out, so the
first record shown can be a few milliseconds younger than the ring holds.

If the display counters keep running during a hang but the marker stops,
the hang is on the ARM side (driver or compositor), not in the display.

## Decoding

| Field | From |
|---|---|
| HVS bits | Linux `vc4_regs.h`, `SCALER_DISPSTAT` and `SCALER_DISPSTATX` (channel 1 feeds pixel valve 2 and HDMI) |
| Temperature | Linux `bcm2835_thermal.c`: 407 °C − 0.538 °C per step, valid bit 10 |
| ARM clock | 19.2 MHz × PLLB multiplier / the ARM channel divider (bits 31:30 of the clock word, rof #67 since Test 530), with the feedback prediv as on the 3B+ and Zero 2 W; an estimate |

## AXI bus counters

With `diag_axi=1` next to `diag_period=` (firmware PR rof #70 and later),
the firmware also reads the two AXI performance monitors of the SoC every
record: three bus watchers on the system bus (HVS, CPU L2, V3D) and three
on the VPU bus (SDRAM, L2 in, DMA uncached). `diagread axi` sums them per
second: read and write transactions and wait cycles, in thousands. High
SDRAM traffic or long waits during a stall point to memory bandwidth.
With them a record is 32 words, so the ring holds about 8 s at 1 ms; use
`diag_period=10000` (10 ms) for 80 s. Register layout and bus names from
Raspberry Pi's Linux `raspberrypi_axi_monitor.c`.

No V3D counters yet; they come when our firmware powers V3D (M5).
