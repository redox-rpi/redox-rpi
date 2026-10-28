# SD card

## State

- `bcm2835-sdhcid` (Arasan controller) is the default driver, patched for
  error recovery and a safe data clock.
- `bcm2835-sdhostd` (sdhost controller, the one Linux uses) is an
  experiment (`sdhost-driver`, `sdhost-boot`): 2.5 times faster writing and
  2.9 times faster reading than `bcm2835-sdhcid`.
- v0.3.0-rc1, an SD card image with sdhost as the only driver and the root
  filesystem on the card, passed on a real Pi 3B+ on 2026-10-28.
- SD card stress test (`scripts/sd-stress.py`, 2026-10-28): both drivers
  pass on the same commit (256 MiB, 1000 small files, 5 restarts, no file
  lost or changed); 256 MiB at 3.2 MB/s with sdhcid, 5.7 MB/s with sdhost.

## Next

- [#6](https://github.com/redox-rpi/redox-rpi/issues/6) sdhost as the default driver: real core clock,
  experiments to base patches, a check on the Zero 2 W.
- [#16](https://github.com/redox-rpi/redox-rpi/issues/16) RedoxFS writes 36 times the data for small writes.

## Later

- sdhost with DMA instead of PIO, after the DMA plan in [#21](https://github.com/redox-rpi/redox-rpi/issues/21)
  (milestone v0.4.0).
- sdhost for the card frees the Arasan controller for Wi-Fi.
