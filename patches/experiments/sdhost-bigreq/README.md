# Experiment: larger sdhost commands (`sdhost-bigreq`)

Issue: [#155](https://github.com/redox-rpi/redox-rpi/issues/155), card write
speed.

`bcm2835-sdhostd` split every request into commands of at most 128 blocks
(64 KiB). Every write command ends with CMD12 and a wait while the card
programs the data, so a 128 KiB RedoxFS record cost two waits and a 1 MiB
write sixteen. Linux sends up to 512 KiB per command to the same controller
(`max_req_size = 524288` in `drivers/mmc/host/bcm2835.c`).

- **base `0001`:** up to 1024 blocks (512 KiB) per read or write command.
  `SDHOSTD_MAX_BLOCKS=<n>` in the driver's environment sets another limit,
  for speed tests.

```bash
scripts/build-image.sh --experiment sdhost-bigreq --experiment sdhost-stats
```

## Why

Pi test 175 (2026-11-01, PR #73's write coalescing): 4 KiB writes became
275 commands of 37 blocks on average for 4 MiB, 1022 ms in the driver and
397 ms of it waiting for the card. For 1 MiB writes the driver moved about
19 MB/s while not waiting, so the bus isn't the limit; the number of waits
is.

## Results

Not tested yet.
