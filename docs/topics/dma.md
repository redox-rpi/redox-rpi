# DMA

How drivers on the Pi 3B+ and Zero 2 W move data with DMA, so that each
driver does not invent its own way. Plan for v0.4.0,
[#21](https://github.com/redox-rpi/redox-rpi/issues/21).

## State

On `main`, no driver uses the BCM2835 DMA controller yet (the sdhost
prototype below is on its own branch). Data moves like this today:

- **sdhost** (SD card) moves every word through its 16-word FIFO with the
  CPU (PIO): about 8 MB/s writing, and the CPU is busy for all of it.
- **dwc2d** (USB) is a bus master and reads and writes memory itself.
- **vesad/fbcond** draw into the firmware's framebuffer with the CPU.

What Redox offers drivers:

- `common::dma::Dma<T>` (base, `drivers/common/src/dma.rs`) allocates
  physically contiguous memory and gives its physical address. On aarch64
  it maps that memory uncached (`MemoryType::Uncacheable`, the kernel's
  Normal non-cacheable), so the CPU and a device always see the same data.
- `common::physmap` maps device registers (`MemoryType::DeviceMemory`).
- There is no cache maintenance for user space: no syscall, and EL0 may
  not run `dc cvac`/`dc civac` (`SCTLR_EL1.UCI` is off). The kernel only
  cleans and invalidates for code it loads (experiment `icache-sync`,
  [#3](https://github.com/redox-rpi/redox-rpi/issues/3)).
- `dwc2d` adds `0xC000_0000` to physical addresses by hand
  (`DMA_BUS_OFFSET`); `rpi-firmwared` does the same for its mailbox
  buffer.

## Design

### Two kinds of DMA

1. **A device's own DMA (bus master):** the device reads and writes memory
   itself (`dwc2d`).
2. **The BCM2835 DMA controller:** a separate engine with 16 channels that
   copies for devices without DMA of their own: sdhost, SPI, I2S/PWM audio,
   and memory to memory (2D copies and fills). Linux uses it for all of
   these.

Both need bus addresses (1); case 2 needs the rest of this page.

### 1. Bus addresses versus physical addresses

Devices and the DMA controller do not see ARM physical addresses. They use
VideoCore bus addresses:

| What | ARM physical | Bus address |
|---|---|---|
| RAM (uncached alias, bypasses the VideoCore L2) | `0x0000_0000`… | `0xC000_0000` + physical |
| Peripherals | `0x3F00_0000`… | `0x7E00_0000`… |

The devicetree says this: `/soc` has `dma-ranges = <0xc0000000 0x0
0x3f000000>` for RAM and `ranges` for the peripherals.

**Decision:** a helper in `common` (for example `common::bus::BusMap`),
built once from the devicetree, with `ram_to_bus(phys)` and
`mmio_to_bus(phys)`. Every driver uses it instead of its own constant;
`dwc2d` and `rpi-firmwared` move to it when they are next changed. Without
`dma-ranges` it falls back to `0xC000_0000`. The VideoCore L2 alias
(`0x4000_0000`) is not used: the uncached alias is what `dwc2d` and Linux
use on the Pi 3.

### 2. Cache maintenance

The DMA controller does not see the ARM caches. Either the buffer is
uncached, or the driver cleans the cache before the device reads and
invalidates it after the device wrote.

**Decision, step 1:** uncached `Dma<T>` buffers, as today. A driver that
transfers to or from a caller's buffer copies through an uncached bounce
buffer of its own (sdhost: 64 KiB, the largest request it splits into).
This needs no kernel change. The copy costs CPU time, but much less than
PIO.

**Step 2, when a user needs it:** cached buffers with clean/invalidate.
The framebuffer blits need it: the display server draws into a cached
back buffer, and drawing into uncached memory is several times slower.
Two ways, to decide then:

- the kernel sets `SCTLR_EL1.UCI`, and drivers run `dc cvac` (clean) and
  `dc ivac`/`dc civac` (invalidate) on their buffers themselves; cheap, but
  every process may then clean and invalidate any of its memory;
- a syscall or a `memory:` scheme call "clean/invalidate this range", as
  `icache-sync` does for code in the kernel; slower per call, but the
  kernel keeps control.

Rule for both: clean before the DMA reads memory, invalidate after it has
written, and invalidate a receive buffer before handing it to the device
too (so no dirty line is written back over the data later). Cache lines
are 64 bytes; buffers start and end on a line boundary.

### 3. The DMA controller and the channel mask

The controller is at bus `0x7E00_7000` (ARM `0x3F00_7000`): channel *n* at
`n × 0x100`, the interrupt status at `0xFE0`, the global enable bits at
`0xFF0`. Channel 15 is elsewhere (`0x7EE0_5000`) and stays unused.
Channels 0–6 are full channels; 7–14 are "lite" channels (at most 64 KiB
per control block, about half the bandwidth, no 2D mode).

The firmware uses some channels itself. The free ones are in the
devicetree, `brcm,dma-channel-mask` on `/soc/dma@7e007000` (`0x7f35` on
the Pi 3B+: channels 0, 2, 4, 5 and 8–14). The firmware reports the same
mask over the mailbox (tag `0x0006_0001`), which `rpi-firmwared` can offer
as a file later.

A transfer is a chain of 32-byte control blocks (aligned to 32 bytes):
`TI` (what to do: increment, DREQ, width, interrupt), source, destination,
length, 2D stride, next block. Devices pace it through their DREQ line
(`TI.PERMAP`), so the controller copies only when the device's FIFO has
room or data.

### 4. User-space interface

All 15 channels share one 4 KiB register page, so the kernel cannot give a
driver only its own channel with `physmap`.

**Decision, step 1:** each driver maps the page itself and uses one fixed
channel from this table, after checking that the channel is in the
devicetree mask (and falls back to the CPU if it is not). It touches only
its own channel and its own bit in the global enable register
(read-modify-write at `0xFF0`). Completion: the channel's own interrupt
line (`dma0`–`dma14` in the devicetree, through `/scheme/irq`), with
polling of `CS.END` as the fallback; or polling only. On the Pi, sdhost
used the same CPU time either way (tests #75 and #87), so the interrupt is
optional.

| Channel | Driver |
|---|---|
| 4 | `bcm2835-sdhostd` |
| 5 | framebuffer blits (display driver) |
| 2, 0 | free, for SPI or audio |

The code for this goes into one small module in `common` (for example
`common::bcm2835_dma`): find the controller and mask in the devicetree,
map the page, start a control block chain, wait for the end with a timeout,
reset the channel after an error.

**Step 2, once a third user or interrupts are needed:** a daemon
`bcm2835-dmad` with a scheme `/scheme/dma`:

- `open("/scheme/dma/channel")` reserves a free channel from the mask (or
  `channel-lite` for a lite one), closing the handle frees it;
- `write` of the bus address of a control block chain starts it; `read`
  returns the channel status when the chain has ended; `fevent` reports
  the end, from the channel's interrupt (`dma0`–`dma14` in the devicetree,
  through `/scheme/irq`);
- the daemon checks that the control blocks point only at memory the
  caller owns or at the device the channel was opened for. Until that
  check exists, only trusted drivers get the scheme, as with `physmap`.

### 5. Errors

After each transfer the driver checks `CS.ERROR` and the `DEBUG` register
(read error, FIFO error, last not set). On an error it resets the channel
(`CS.RESET`), reports it, and repeats the request without DMA (PIO for
sdhost, the CPU for blits). A transfer that does not end within a timeout
is aborted (`CS.ABORT`, then `CS.RESET`) the same way.

## First users

### sdhost

- Read: control block with `TI` = `PERMAP(13)` (sdhost's DREQ, `dmas =
  <&dma 13>` in the devicetree), `SRC_DREQ`, `DEST_INC`, `WAIT_RESP`;
  source `SDDATA` at bus `0x7E20_2040`, destination the bounce buffer.
  Write: `DEST_DREQ`, `SRC_INC`, source the bounce buffer, destination
  `SDDATA`.
- sdhost has no DMA enable bit: it raises DREQ by the FIFO thresholds in
  `SDEDM`, which the driver already sets to 4 as Linux does.
- Linux does not let the DMA read the last words of a multi-block read:
  sdhost handles the FIFO DREQs badly there, so the last `(read threshold
  − 1)` = 3 words come by PIO (`drain_words` in Linux
  `drivers/mmc/host/bcm2835.c`). The driver does the same.
- Linux uses PIO for single-block requests (`PIO_THRESHOLD` 1) and DMA
  above. Start the same way; measure whether the limit should be higher.
- Measure with `scripts/sd-stress.py` (MB/s) and the `ps` CPU time of
  `bcm2835-sdhostd` during a 256 MiB write, against PIO.

### Framebuffer blits (graphics session, [#33](https://github.com/redox-rpi/redox-rpi/issues/33))

- 2D mode (`TI.TDMODE`, full channels only): `TXFR_LEN` holds rows and
  bytes per row, `STRIDE` the source and destination gaps, so one control
  block copies a rectangle from a back buffer into the framebuffer for
  `MODE_DIRTYFB`.
- Fills: source a 32-byte pattern with `SRC_INC` off.
- No DREQ (`PERMAP` 0): memory to memory runs at full speed.
- The framebuffer is firmware memory. Its bus address is what the mailbox
  returned; Redox gets the ARM address from the bootloader
  (`FRAMEBUFFER_ADDR`) and converts it with the `BusMap`.
- Needs cache maintenance step 2 for a cached back buffer; until then the
  back buffer is a `Dma` buffer (uncached), which works but draws slowly.

## Prototype results (sdhost)

Experiment `sdhost-dma` (SD card session, branch `sdhost-dma`, issue
[#30](https://github.com/redox-rpi/redox-rpi/issues/30)): step 1 of this
plan, channel 4, a 64 KiB uncached bounce buffer, PIO for single blocks and
the last 3 words of multi-block reads.

- QEMU: writes and reads of random files match the card image. QEMU copies
  a whole control block at once and ignores DREQ pacing, so the timing and
  the drain words needed the real Pi.
- Pi tests #42 (polling), #75 (interrupt) and #87 (polling again, for CPU
  time): **data safe** (hash after a reset, `sd-stress.py` 0 lost), and the
  same speed as PIO: 256 MiB written at 7.7 MB/s, read at 18 MB/s. The card
  limits the speed, not the CPU.
- CPU time of `bcm2835-sdhostd` (SCHTIME) for 256 MiB: writing 3.4–8.6 s
  per 35 s with polling and 4.3 s with the interrupt; reading about 1.0 s
  per 15 s either way (7 %). With PIO the driver is busy for the whole
  transfer. So DMA frees the CPU, and the interrupt adds no clear gain.

## In progress

- sdhost DMA towards a PR (SD card session, #30), on this plan as it is:
  channel 4, bus offset from `/soc` `dma-ranges` (fallback `0xC000_0000`),
  the DMA code in one module so it can move to `common` later.

## Next

- `common::bus::BusMap` and `common::bcm2835_dma` out of the sdhost driver
  when the display driver needs them (blits, channel 5).
- Decide cache maintenance step 2 with the graphics session.

## Later

- `bcm2835-dmad` and `/scheme/dma` (step 2 of the interface), with
  interrupts.
- DMA for SPI, I2S and PWM audio when a driver needs it.
