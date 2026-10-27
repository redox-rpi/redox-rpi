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
/scheme/initfs/bin/bcm2835-sdhostd disk.sdhost &
```

With a scheme name it runs in the foreground (hence the `&`); without one
it expects to be started by `init`. From then on, the card slot is wired to
sdhost: `bcm2835-sdhcid` and `/scheme/data` no longer reach the card until
the next boot.

## Manual test on the Pi

1. Before starting the driver, hash the first 8 MiB of the boot partition
   through the old driver:
   `dd if=/scheme/disk.mmc/0p0 bs=4096 count=2048 | sha256sum`
   (`sha256sum` on the partition fails with "Value too large", and `head -c`
   reads nothing from it; a result of `e3b0c442…` is the hash of no data).
2. Start the new driver (command above). It prints the controller address,
   the card (type, size, bus width, clock) and `serving /scheme/disk.sdhost`.
3. `ls /scheme/disk.sdhost` lists the disk and its partitions.
4. `dd if=/scheme/disk.sdhost/0p0 bs=4096 count=2048 | sha256sum` must give the same
   hash as step 1.
5. Reboot to get the old driver back.

If step 2 hangs or the system freezes, note the last line the driver
printed.

## Results

- QEMU, SDSC and SDHC cards: works (4-bit bus, 25 MHz).
- QEMU, started by hand (2026-10-27): finds the card and serves
  `disk.sdhost`; it then crashed in the daemon helper, which expects `init`
  (fixed: no daemon when started with a scheme name; the next QEMU run
  started it without a crash). In QEMU the root
  filesystem is on the card, so it is gone once sdhost takes the slot; on
  the netbooted Pi the root is in RAM.
- **Real Pi 3B+, 2026-10-27, started by hand** (build
  `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver`): works.
  `SDHC/SDXC card, 15193 MiB, 4-bit bus, up to 25 MHz`, serves
  `disk.sdhost` with partitions `0p0` and `0p1`. Read through it: the
  partition table matches the SD image the card was made from (except the
  disk signature, new on every `make-sdcard.sh` run), the FAT boot sector is
  valid (`REDOXBOOT`, `FAT32`). 8 MiB read in 1.85 s, 4.5 MB/s. Writing
  and starting at boot: see [`sdhost-boot`](../sdhost-boot/README.md).
- Real Pi 3B+, 2026-10-27, started at boot instead of `bcm2835-sdhcid`
  (commit `2c66e72`): the Pi did not come back on the network. Likely cause:
  without `disk.mmc`, mounting the data partition (`60_data.service`) waits
  forever and blocks the boot before devshd. Not confirmed, no console log.
  Hence the manual start.

- **High speed mode** (patch `0018`, real Pi 3B+, 2026-10-27, build
  `v0.1.0-alpha-61-g90ef0ee+ldso-diagnostics+sdhost-driver+sdhost-boot+sdhost-stats`): the card switches to high speed, the driver aims at 50 MHz.
  8 MiB written, the Pi restarted, the same SHA-256 read back from the card.

  | | 25 MHz target (15.6 MHz) | 50 MHz target (31 MHz) |
  |---|---|---|
  | write, 1 MiB per call, `conv=fsync` | 4.4 MB/s | 6.7 MB/s |
  | write, 64 KiB per call | 2.0 MB/s | 3.2 MB/s |
  | write, 4 KiB per call | 130 kB/s | 181 kB/s |
  | read 8 MiB after a restart (from the card) | 4.5 MB/s | 13 MB/s |

  Writes from `/scheme/rand`, which gives 14 MB/s, so the 1 MiB rate is
  partly the random source.

The clock divider assumes a 400 MHz core clock, the most the firmware may
switch to; the firmware reports 250 MHz on this Pi, so the card runs at
31 MHz instead of 50. Reading the real core clock (mailbox,
`rpi-firmware`) and following its changes could give the rest.

Small writes are slow because of RedoxFS, not the driver: the statistics
(`sdhost-stats`) show that 1 MiB written in 4 KiB calls sent about 2600
write requests with 74,000 blocks (37 MB) to the card, 36 times the data.
The driver moved them at bus speed; a third of the time was the card
programming. See task T16.
