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

Pi test 177 (2026-11-01, image C = PR #73 + this experiment, against the
#175 images A = main and B = PR #73): **works, but no measurable gain.**
Every sha256 matches, and no `write of block` or `read of block` error on
serial, so the controller takes more than 128 blocks per command.

- For a 1 MiB write RedoxFS hands the driver whole 128 KiB records (256
  blocks) plus one 8-block write per metadata block: 16 MiB became 128
  requests of 256 blocks and 121 of 8 (33736 blocks). With this experiment
  each record is one command instead of two.
- 1 MiB × 5 on a fresh boot each: A 4.5–11 MB/s, B 5.3–11 MB/s, C 4.1–9.6
  MB/s. The card's own pauses scatter the results more than the number of
  commands changes them.
- C: 4 KiB writes 3.9 MB/s (B 2.7–2.9), 64 KiB 3.1 and 5.3 MB/s (B
  6.1–6.5), reading 64 MiB 20 MB/s.

So the number of commands is not where the time goes. The experiment stays
here, not in the default patches.
