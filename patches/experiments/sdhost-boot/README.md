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

None yet.
