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
period and writes one record into a 1 MB ring at the end of the
VideoCore's memory, which no ARM kernel uses. The firmware puts the ring's
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
thermal sensor, the PLLB word (ARM clock) and the marker.

## What `stalls` and `watch` report

| Event | Meaning |
|---|---|
| `stall hdmi frames` | the HDMI frame count stood still: the display stopped scanning out |
| `stall hvs1 frames` | the HVS channel 1 frame count stood still |
| `stall marker` | the marker stood still, once it moved at all (a driver that writes the marker on each flip: no flips) |
| `gap` | the sampler itself had no record: the VPU was busy |
| `flag` | an HVS error bit went from 0 to 1 (underflow, line or frame error, COB low, DMA error); they stay set until written back |
| `clock` | the ARM clock changed (governor or throttling) |

If the display counters keep running during a hang but the marker stops,
the hang is on the ARM side (driver or compositor), not in the display.

## Decoding

| Field | From |
|---|---|
| HVS bits | Linux `vc4_regs.h`, `SCALER_DISPSTAT` and `SCALER_DISPSTATX` (channel 1 feeds pixel valve 2 and HDMI) |
| Temperature | Linux `bcm2835_thermal.c`: 407 °C − 0.538 °C per step, valid bit 10 |
| ARM clock | 19.2 MHz × PLLB multiplier, with the feedback prediv and `pllb_arm` / 2 as on the 3B+ and Zero 2 W; an estimate |

No V3D counters and no AXI bus counters yet; they come with later
firmware steps of #66.
