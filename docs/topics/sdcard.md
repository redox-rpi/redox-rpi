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

## Later

- [#16](https://github.com/redox-rpi/redox-rpi/issues/16) RedoxFS writes 28 times the data for 4 KiB writes
  (milestone v0.4.0). The same on x86_64 as on aarch64 (QEMU,
  `scripts/redoxfs-write-amp.py`, RedoxFS 0.9.1 `c988cb8`, the newest
  upstream), so an upstream problem; the user reports it to Redox. Related upstream work: RedoxFS
  [#51](https://gitlab.redox-os.org/redox-os/redoxfs/-/work_items/51) and
  [#57](https://gitlab.redox-os.org/redox-os/redoxfs/-/work_items/57).
- [#24](https://github.com/redox-rpi/redox-rpi/issues/24) a small release image that grows to the whole card on the
  first boot, with a welcome app (milestone v0.4.0).
- sdhost with DMA instead of PIO, after the DMA plan in [#21](https://github.com/redox-rpi/redox-rpi/issues/21)
  (milestone v0.4.0).
- sdhost for the card frees the Arasan controller for Wi-Fi.
