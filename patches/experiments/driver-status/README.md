# Experiment `driver-status`

Drivers offer a read-only `status` file in their scheme, per
`docs/process/coding-standards.md` ("Drivers: a status file in the
scheme"). The first ones:

- **driver-block:** the `Disk` trait gets `status()`, which returns `None` by default.
  `DiskScheme` serves `/scheme/<disk scheme>/status` with the text of every
  disk that has one. It is not in the directory list, because tools that list
  a disk scheme open every entry as a disk.
- **bcm2835-sdhostd** (`/scheme/disk.mmc/status`, Pi 0-3 SD card):
  - card type, size, bus width and high speed mode;
  - SD clock and the highest core clock;
  - SDCMD, SDCDIV, SDHSTS, SDHCFG and SDEDM, with the state machine and FIFO fill;
  - counters since start: requests, blocks and time per direction, write busy
    time, errors by kind and SDHSTS bit, retries, recoveries, re-inits,
    failed requests, and the last error with its age.

Reading the file reads registers only. It doesn't write, wait or log.

## Test

- QEMU `raspi3b`: `cat /scheme/disk.mmc/status` after boot.
- Pi 3B+: read it at idle and after a large read from the card, and check
  that the counters grow and the clock matches the boot line.
