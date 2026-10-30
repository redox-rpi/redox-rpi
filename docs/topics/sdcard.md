# SD card

## State

- `bcm2835-sdhostd` (sdhost controller, the one Linux uses) is the default
  SD driver (base patches `0025`–`0028`, #6): 2.5 times faster writing and
  2.9 times faster reading than `bcm2835-sdhcid`, which no longer starts.
  `0028` divides the SD clock for the firmware's highest core clock (from
  `rpi-firmwared`). Transfer statistics at boot stay an experiment
  (`sdhost-stats`, `0029`).
- Kernel `0008` frees a thread that ends itself when its syscall returns
  (was experiment `ctx-leak-fix`, #31); without it the kernel ran out of
  memory under SD load.
- Pi test #61 (2026-10-29, `v0.3.0-dev-212-gc9fcc6d+x12cb`): standard
  checks and `scripts/sd-stress.py` with 2 restarts pass, no file lost or
  changed.
- v0.3.0-rc1, an SD card image with sdhost as the only driver and the root
  filesystem on the card, passed on a real Pi 3B+ on 2026-10-28.
- SD card stress test (`scripts/sd-stress.py`, 2026-10-28): both drivers
  pass on the same commit (256 MiB, 1000 small files, 5 restarts, no file
  lost or changed); 256 MiB at 3.2 MB/s with sdhcid, 5.7 MB/s with sdhost.

## Next

- [#6](https://github.com/redox-rpi/redox-rpi/issues/6) and [#31](https://github.com/redox-rpi/redox-rpi/issues/31): branch `sdhost-ctxleak` goes to main
  through a pull request. #24 (Zero 2 W) waits for a later release.

## Later

- [#16](https://github.com/redox-rpi/redox-rpi/issues/16) RedoxFS writes 28 times the data for 4 KiB writes
  (milestone v0.4.0). The same on x86_64 as on aarch64 (QEMU,
  `scripts/redoxfs-write-amp.py`, RedoxFS 0.9.1 `c988cb8`, the newest
  upstream), so an upstream problem; the user reports it to Redox. Related upstream work: RedoxFS
  [#51](https://gitlab.redox-os.org/redox-os/redoxfs/-/work_items/51) and
  [#57](https://gitlab.redox-os.org/redox-os/redoxfs/-/work_items/57).
  Cause: every `write` call is a transaction that rewrites its whole
  128 KiB record plus about 7 metadata blocks. Prototype
  `redoxfs-write-coalesce` (branch `t16-write-amp`) collects consecutive
  small writes into whole records: 28.4 → 1.3 in QEMU; Pi test #46.
- [#24](https://github.com/redox-rpi/redox-rpi/issues/24) a small release image that grows to the whole card on the
  first boot, with a welcome app (milestone v0.4.0). Welcome app:
  experiment `welcome` (branch `welcome-app`) asks for keyboard layout, time
  zone, hostname, root password and Wi-Fi on the first boot, before login;
  passes in QEMU and on the Pi (test 78), pull request open.
- sdhost with DMA instead of PIO, after the DMA plan in [#21](https://github.com/redox-rpi/redox-rpi/issues/21)
  (milestone v0.4.0): prototype `sdhost-dma` (branch `sdhost-dma`) passes
  in QEMU; Pi test #42.
- sdhost for the card frees the Arasan controller for Wi-Fi.
