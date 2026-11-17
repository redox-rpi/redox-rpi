# Experiment genet-coalesce: fewer interrupts from the Pi 400 Ethernet

Issue #540 (REQ-NET-8, criterion 3). Needs the experiment `pi400`, whose
`base/0002` adds `bcmgenetd`.

A 100 MiB TCP download on the Pi 400 reached 11.0 MB/s with curl and
12.8 MB/s with nc into `/scheme/null` (Test 485), against a proposed
20 MB/s. `bcmgenetd` took one interrupt per received frame and one per
sent frame (the ACKs of a download), so the driver and netstack woke up
about one and a half times per frame. Each wakeup goes through the
kernel's user-space interrupt path and the scheme to netstack.

- `base/0100`: the receive ring (default queue 16) interrupts once per 16
  frames, or 100 µs after the first frame not yet signalled
  (`DMA_MBUF_DONE_THRESH` and `DMA_RING16_TIMEOUT`, as Linux
  `bcmgenet_set_rx_coalesce`). The transmit done interrupt stays masked:
  `transmit()` reads the consumer index itself and never waits for an
  interrupt. A ping gets at most 100 µs longer.

The 100-ms poll of the driver stays as the fallback for a lost interrupt.

## Not changed

- The receive copy: DMA memory is Normal non-cacheable on aarch64
  (`common::dma`), so each frame is copied out of uncached memory. At
  about 1.5 µs per frame that is small next to the wakeups.
- One ring with 256 descriptors in each direction: no frames were lost in
  Test 485 (the SHA-256 matched), so the ring is not the limit.

## Test

Unit test in `ring.rs` (register offsets and values, mask). QEMU has no
GENET (raspi4b in QEMU 10.2), so the effect is measured on the Pi 400:
the same transfers as Test 485, with CPU time per process
(`/scheme/sys/context`) and interrupt counts (`/scheme/sys/irq`) before
and after, on this image and on the image of Test 485.
