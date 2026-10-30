# sdhost-dma

Prototype for [#21](https://github.com/redox-rpi/redox-rpi/issues/21):
`bcm2835-sdhostd` moves transfers of more than one block with the BCM2835
DMA controller, step 1 of the [DMA plan](../../../docs/topics/dma.md)
(branch `dma-plan`).

Needs the sdhost base patches (`sdhost-default`); this branch is based on
`sdhost-ctxleak`.

- Channel 4, if `brcm,dma-channel-mask` in the devicetree leaves it free;
  otherwise the driver prints `no DMA (...)` and moves everything with the
  CPU as before.
- One control block per transfer (at most 128 blocks, 64 KiB), paced by
  sdhost's DREQ 13, through a 64 KiB uncached bounce buffer.
- `0002`: the driver sleeps until the channel interrupt (`dma4` in the
  devicetree, opened from the kernel's `irq` scheme before `setrens`) and
  checks `CS` at least every 10 ms. If the interrupt cannot be opened it
  prints `cannot use DMA interrupt ... polling` and polls `CS.END` as before.
- Single blocks, the last 3 words of a multi-block read (as Linux does,
  `drain_words`) and the retry after a failed transfer go through the FIFO
  with the CPU.
- `SDHOSTD_NO_DMA` in the environment turns DMA off, for comparisons.

Boot log: `bcm2835-sdhostd: DMA interrupt /scheme/irq/phandle-1/1,20`,
`bcm2835-sdhostd: DMA channel 4 at 0x3f007000`.

QEMU copies a whole control block at once and ignores DREQ pacing, so a
QEMU run checks the setup and the data, not the timing.

## Results

QEMU raspi3b, 2026-10-29, image `v0.3.0-dev-122-g74a1e24+x607a`
(experiments ctx-leak-fix rtl8152 sdhost-dma timesync), root on QEMU's SD
card:

- Boot log `DMA channel 4 at 0x3f007000`, no DMA errors or fallbacks.
- Write: the guest wrote 8 MiB (`bs=64k`) and 5000000 bytes (not a multiple
  of 512) of random data with `conv=fsync`; read back on the host from the
  card image with `tools/redoxfs-file`, both SHA-256 match.
- Read: the host put new random data into both files; after a fresh boot
  the guest's SHA-256 match.

Pi 3B+, test 42 (`0001` only): data good, 8.8 MB/s write and 18 MB/s
read, but the driver used the CPU for 80–90 % of the transfer time while
polling. Hence `0002`.

QEMU raspi3b, 2026-10-29, image `v0.3.0-dev-218-g8deec9e+xbe97`
(experiments sdhost-dma timesync, base from `sdhost-ctxleak`): boot log
`DMA interrupt /scheme/irq/phandle-1/1,20`; the same write test (8 MiB and
5000000 bytes, `conv=fsync`), both SHA-256 match on the host.

Pi 3B+, test 75 (`0002`, `v0.3.0-dev-218-g8deec9e+xbe97`): boot log shows
the interrupt, no DMA errors; 256 MiB write 7.7 MB/s, read 18 MB/s (as with
polling); data intact after a reset and over `sd-stress.py --cycles 2`.
CPU time of the driver (SCHTIME, the real one): +4.3 s for the 35 s write
(12 %), +1.1 s for the 15 s read (7 %). ACTTIME, which test 42 used, rises by
82 % and 92 % because it also counts the time the core sits idle after the
driver blocked: with nothing else to run, the kernel does not switch away
from the last context.

Open: SCHTIME with polling (test 87), to put a number on the saving. (DREQ pacing, drain words, MB/s and CPU time against PIO with
`scripts/sd-stress.py`).
