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
  sdhost's DREQ 13, through a 64 KiB uncached bounce buffer; the driver
  polls `CS.END`.
- Single blocks, the last 3 words of a multi-block read (as Linux does,
  `drain_words`) and the retry after a failed transfer go through the FIFO
  with the CPU.
- `SDHOSTD_NO_DMA` in the environment turns DMA off, for comparisons.

Boot log: `bcm2835-sdhostd: DMA channel 4 at 0x3f007000`.

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

Open: the Pi (DREQ pacing, drain words, MB/s and CPU time against PIO with
`scripts/sd-stress.py`).
