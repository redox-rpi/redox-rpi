# Experiment: start the sdhost driver at boot

Builds on [`sdhost-driver`](../sdhost-driver/README.md): `bcm2835-sdhostd`
replaces `bcm2835-sdhcid` in the initfs services and serves the card as
`disk.mmc`, so the data partition (network boot) or the root filesystem
(SD card image) is mounted through the sdhost controller.

```bash
scripts/build-image.sh --experiment ldso-diagnostics --experiment sdhost-driver --experiment sdhost-boot
```

`sdhost-boot` needs `sdhost-driver` before it on the command line.

## Test on the Pi (network boot)

1. Boot. The boot log on the screen shows the `bcm2835-sdhostd:` lines
   (controller, card, `serving /scheme/disk.mmc`). If the boot stops, note
   the last lines on the screen.
2. `scripts/pi-test.py` standard checks; its `sdcard` check writes and reads
   a file on `/scheme/data`, now through sdhost.
3. Write test across drivers: write 4 MiB of random data and its hash
   through sdhost:
   `dd if=/scheme/rand of=/scheme/data/sdhost-4m.bin bs=4096 count=1024`
   and `sha256sum /scheme/data/sdhost-4m.bin`.
4. Boot the stable image (old driver) and hash the file again: the same
   hash means sdhost wrote correctly. Delete the file afterwards.

## Results

2026-10-27, real Pi 3B+ over netboot, `v0.1.0-alpha-52-ga4ba2e3+ldso-diagnostics+sdhost-driver+sdhost-boot`:
**works**. [Report](../../../docs/test-reports.md).

- Boot: `bcm2835-sdhostd` finds the card (SDHC, 15193 MiB, 4-bit bus) and
  serves `disk.mmc`; `/scheme/data` is mounted through it. Booted three
  times, no hang. The earlier hang with sdhost at boot did not come back.
- Write: 4 MiB from `/scheme/rand` to `/scheme/data` through sdhost, then
  the image with `bcm2835-sdhcid` read the same SHA-256
  (`2fae7622…31522`). A small text file also matched.
- 50 program starts, none failed.
- Write speed is low: 4 MiB in 33 s, 127 kB/s (reading: 4.5 MB/s). Next
  step for the driver.
- The firmware reports a core clock of 250 MHz, not the 400 MHz the clock
  divider assumes, so the card runs at about 15.6 MHz instead of 25 MHz.

In QEMU the image behaves like the one with `bcm2835-sdhcid`: QEMU's card
is the network boot image itself, which has no data partition, so the
`/scheme/data` mount fails with both drivers.
