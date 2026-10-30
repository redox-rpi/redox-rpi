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
- `rename` works on every RedoxFS, not only the root one
  ([#57](https://github.com/redox-rpi/redox-rpi/issues/57)): redoxfs `0036`,
  default with branch `redoxfs-rename`; before, `mv` on
  `/scheme/data` failed with os error 2. Pi test 83 passed on 2026-10-30.
- RedoxFS collects small consecutive writes into whole records
  ([#16](https://github.com/redox-rpi/redox-rpi/issues/16)): redoxfs
  `0037`–`0039`, default with branch `t16-write-amp`. Before, every `write`
  rewrote its 128 KiB record plus about 7 metadata blocks, 28 times the data
  for 4 KiB writes; now 1.3 times. On the Pi (test 70, 2026-10-30) 1 MiB
  copied at `bs=4k` takes 0.49 s instead of 12.7 s; data waits at most 2 s.
  Still an upstream problem (RedoxFS
  [#51](https://gitlab.redox-os.org/redox-os/redoxfs/-/work_items/51),
  [#57](https://gitlab.redox-os.org/redox-os/redoxfs/-/work_items/57)); the
  user reports it to Redox.

## Next

- [#6](https://github.com/redox-rpi/redox-rpi/issues/6) sdhost as the default driver, branch `sdhost-default`:
  `sdhost-driver` and `sdhost-boot` are base patches `0022`–`0024`, and
  `0025` divides the SD clock for the firmware's highest core clock (from
  `rpi-firmwared`) instead of a fixed 400 MHz. Passes in QEMU
  (`v0.3.0-dev-76-g13092c5+x9f76`). Pi tests queued: #23 network boot,
  #24 SD card image on the Pi 3B+ and Zero 2 W (needs the user). Merge to
  main after those.

## Later

- [#24](https://github.com/redox-rpi/redox-rpi/issues/24) a small release image that grows to the whole card on the
  first boot, with a welcome app (milestone v0.4.0).
- sdhost with DMA instead of PIO, after the DMA plan in [#21](https://github.com/redox-rpi/redox-rpi/issues/21)
  (milestone v0.4.0): prototype `sdhost-dma` (branch `sdhost-dma`) passes
  in QEMU; Pi test #42.
- sdhost for the card frees the Arasan controller for Wi-Fi.
