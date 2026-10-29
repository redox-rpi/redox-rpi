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
