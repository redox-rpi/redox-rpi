# Experiment: bcm2835-sdhostd, a driver for the sdhost SD controller

New driver for the BCM2835 sdhost controller (`brcm,bcm2835-sdhost`), the
one Linux uses for the card slot. It routes GPIO 48–53 to the controller,
initializes the card and serves it as a disk scheme.

```bash
scripts/build-image.sh --experiment sdhost-driver
```

The image still boots with `bcm2835-sdhcid` (Arasan controller). The new
driver is in the initfs and is started by hand, under its own scheme name:

```bash
/scheme/initfs/bin/bcm2835-sdhostd disk.sdhost
```

From then on, the card slot is wired to sdhost: `bcm2835-sdhcid` and
`/scheme/data` no longer reach the card until the next boot.

## Manual test on the Pi

1. Before starting the driver, hash the boot partition through the old
   driver: `sha256sum /scheme/disk.mmc/0p0`
2. Start the new driver (command above). It prints the controller address,
   the card (type, size, bus width, clock) and `serving /scheme/disk.sdhost`.
3. `ls /scheme/disk.sdhost` lists the disk and its partitions (`0`, `0p0`,
   `0p1`).
4. `sha256sum /scheme/disk.sdhost/0p0` must give the same hash as step 1.
5. Reboot to get the old driver back.

If step 2 hangs or the system freezes, note the last line the driver
printed.

## Results

- QEMU, SDSC and SDHC cards: works (4-bit bus, 25 MHz).
- Real Pi 3B+, 2026-10-27, started at boot instead of `bcm2835-sdhcid`
  (commit `2c66e72`): the Pi did not come back on the network. Likely cause:
  without `disk.mmc`, mounting the data partition (`60_data.service`) waits
  forever and blocks the boot before devshd. Not confirmed, no console log.
  Hence the manual start.

The clock divider assumes a 400 MHz core clock; at 250 MHz the card only
runs slower. The mailbox experiment can provide the real rate.
