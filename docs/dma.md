# DMA on the Raspberry Pi

How drivers on the Pi 3B+ and Zero 2 W move data with DMA, so that each
driver does not invent its own way. Issue
[#21](https://github.com/redox-rpi/redox-rpi/issues/21).

## Two kinds of DMA

1. **A device's own DMA (bus master).** The device reads and writes memory
   itself. `dwc2d` (USB) does this today.
2. **The BCM2835 DMA controller.** A separate engine with 16 channels that
   copies data for devices that have no DMA of their own: sdhost, SPI,
   I2S/PWM audio and others. Linux uses it for all of these. No driver of
   ours uses it yet; sdhost moves every word with the CPU (PIO).

## What Redox has today

- `common::dma::Dma<T>` (base, `drivers/common/src/dma.rs`) allocates
  physically contiguous memory and gives its physical address.
- On aarch64 that memory is mapped uncached (`MemoryType::Uncacheable`,
  the kernel's `Normal non-cacheable` attribute), so the CPU and the
  device always see the same data. There is no cache maintenance (clean,
  invalidate) for cached buffers.
- There is no DMA engine (case 2) in Redox; x86 does not need one.
- `dwc2d` adds `0xC000_0000` to physical addresses by hand
  (`DMA_BUS_OFFSET`). `rpi-firmwared` does the same for its mailbox
  buffer.

## Decisions

### 1. Bus addresses: one helper, from the devicetree

Devices and the DMA controller do not use ARM physical addresses. They use
VideoCore bus addresses:

| What | ARM physical | Bus address |
|---|---|---|
| RAM (uncached alias, bypasses the VideoCore L2) | `0x0000_0000`… | `0xC000_0000` + physical |
| Peripherals | `0x3F00_0000`… | `0x7E00_0000`… |

The devicetree already says this: `/soc` has
`dma-ranges = <0xc0000000 0x0 0x3f000000>` (RAM) and `ranges` (peripherals).

**Decision:** a helper in `common` (for example `common::bus::BusMap`),
built once from the devicetree, with `ram_to_bus(phys)` and
`mmio_to_bus(phys)`. Every driver uses it instead of its own constant.
`dwc2d` and `rpi-firmwared` move to it when they are next changed. Without
`dma-ranges` (QEMU's `raspi3b` devicetree has one too) the helper falls
back to `0xC000_0000`.

### 2. Caching: uncached buffers first

The DMA controller does not see the ARM caches. Either the buffer is
uncached, or the driver cleans the cache before the device reads and
invalidates it after the device wrote.

**Decision:** use uncached `Dma<T>` buffers, as today. A driver that
transfers to or from a request buffer copies through an uncached bounce
buffer of its own (for sdhost: up to 64 KiB). This needs no kernel
change.

Cached buffers with clean/invalidate come later, and only if measurements
show the bounce copy costs more than it saves. They need either `dc civac`
from user space (the kernel must set `SCTLR_EL1.UCI`) or a syscall, and a
rule for who cleans what. That is a separate issue then.

### 3. Channels: a fixed channel per driver first

The firmware uses some channels itself. The free ones are in the
devicetree, `brcm,dma-channel-mask` on `/soc/dma@7e007000` (`0x7f35` on
the Pi 3B+: channels 0, 2, 4, 5 and 8–14), and from the mailbox (tag
`0x00060001`). Channels 0–6 are full channels; 7–14 are "lite" channels
(at most 64 KiB per control block, half the bandwidth). Channel 15 is at
another address and stays unused.

**Decision:** each driver takes a fixed channel from the free mask, listed
here, and checks at start that the mask still has it:

| Channel | Driver |
|---|---|
| 4 | `bcm2835-sdhostd` |
| 5 | next user (SPI or audio) |

A `bcm2835-dmad` that hands out channels through a scheme comes only when
two drivers might want the same channel, or channels must be shared.

### 4. Completion: polling first, interrupts later

Each channel has its own interrupt (`dma0`–`dma14` in the devicetree). The
channel's `CS.END` bit says that the last control block finished.

**Decision:** the first user polls `CS.END` with `yield` between checks.
sdhost waits for the card anyway: after a transfer the driver checks the
card status, which takes longer than the copy. Interrupts through
`/scheme/irq` come with the interrupt work for `dwc2d`
(branch `dwc2-interrupts`), when a driver must not spin.

### 5. Errors

`CS.ERROR` and the `DEBUG` register (read error, FIFO error, last
not-set) are checked after each transfer. On an error the driver resets
the channel (`CS.RESET`), reports the error and falls back to PIO for
that request. A transfer that does not end within a timeout is aborted
(`CS.ABORT`) the same way.

## First user: sdhost

- Control block (32-byte aligned, in an uncached `Dma` buffer):
  `TI` = `PERMAP(13)` (sdhost's DREQ, `dmas = <&dma 13>` in the
  devicetree), `SRC_DREQ` (read) or `DEST_DREQ` (write), `WAIT_RESP`,
  and increment on the memory side only; source or destination `SDDATA`
  at bus address `0x7E20_2040`.
- sdhost has no DMA enable bit: it raises DREQ by the FIFO thresholds in
  `SDEDM`, which the driver already sets to 4 as Linux does. With DMA,
  Linux turns off the data interrupt (`SDHCFG_DATA_IRPT_EN`) and keeps the
  busy interrupt; to be checked against the datasheet and Linux
  `drivers/mmc/host/bcm2835.c` when writing the driver.
- Data goes through a 64 KiB uncached bounce buffer; larger requests take
  several control blocks or several transfers.
- Measure against PIO with `scripts/sd-stress.py` (MB/s) and `ps` CPU time
  of `bcm2835-sdhostd` during a 256 MiB write. Keep PIO for small requests
  if DMA is not faster for them.

Follow-up issue: sdhost with DMA.

## Not decided here

- DMA for other devices (SPI, I2S, PWM): when a driver needs it.
- The VideoCore L2 cached alias (`0x4000_0000`): not used; the uncached
  alias is what `dwc2d` uses and what Linux uses for the Pi 3.
