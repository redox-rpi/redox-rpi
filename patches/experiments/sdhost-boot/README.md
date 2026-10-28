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

## Test on the Pi (SD card image, root on the card)

Release candidate v0.3.0-rc1 (test #12): `scripts/build-image.sh --sdcard` with
`ldso-diagnostics`, `timesync`, `rtl8152`, `sdhost-driver` and
`sdhost-boot`, then `scripts/make-sdcard.sh --local`. Image:
`http://homenodellm:8080/sdcard/redox-rpi-v0.3.0-rc1.img.xz`, with its
manifest and `SHA256SUMS` next to it.

For the person at the Pi:

1. Write the image to a **spare** SD card (SwiftFlash, or Raspberry Pi
   Imager with "Use custom"). If you use the network boot card instead,
   write `sdcard-netboot.img.xz` back to it afterwards; the other tests in
   the queue need it.
2. Put the card in the Pi 3B+, with HDMI, the serial console and Ethernet
   connected, and switch it on.
3. The bootloader shows the resolution menu: press **Enter** (keyboard or
   serial console), nothing else (`e` opens the environment editor).
4. Report the last lines on the screen if it stops before `redox login:`.

Then over the serial console (the image has no devshd):

5. `cat /etc/redox-rpi-release`: build `v0.2.0-5-gd679bfb+xed14`. The boot
   log shows `bcm2835-sdhostd: ... high speed` and `serving
   /scheme/disk.mmc`, and no `bcm2835-sdhcid`.
6. Write to the root filesystem:
   `dd if=/scheme/rand of=/home/root/rc1.bin bs=1048576 count=8 conv=fsync`,
   `sha256sum /home/root/rc1.bin`.
7. Restart with `echo reset > /scheme/sys/kstop`, log in, hash the file
   again: the same hash. Then 100 program starts
   (`for i in 1..=100; ls / > /dev/null || echo FAILED; end`), and delete
   the file.
8. Optional: boot it twice more, and check that the network comes up
   (`ping` the build server) and timesync sets the clock.

In QEMU (2026-10-28, `v0.2.0-5-gd679bfb+xed14`): boots to the login with
the root filesystem from `/scheme/disk.mmc/0p2` through sdhost (QEMU's card
is `harddrive.img`, partitions `0p0`–`0p2`); 4 MiB written to
`/home/root` with `conv=fsync`. RedoxFS logs "No such file or directory"
for `0`, `0p0` and `0p1` while it looks for the root filesystem: those are
not RedoxFS, it then mounts the next one. The same lines appear with
`bcm2835-sdhcid`.

### Why rc1 of v0.2.0 hung, and why this does not

In v0.2.0 rc1, `bcm2835-sdhostd` was started by hand **next to**
`bcm2835-sdhcid`, with `/scheme/data` mounted through sdhcid. Starting
sdhost switches the card slot pins (GPIO 48-53) away from the Arasan
controller and resets the card, so sdhcid's next access to the mounted
data partition (for example timesync writing `timesync.last`) waits on a
controller without a card, and RedoxFS hangs with it. With `sdhost-boot`,
`bcm2835-sdhcid` is not started at all: one driver owns pins and card from
the start. That setup passed on the Pi with the data partition (three
boots, writes checked across drivers and after restarts).

## Results

2026-10-28, real Pi 3B+, SD card image v0.3.0-rc1 (`v0.2.0-5-gd679bfb+xed14`), root
filesystem on the card: **works** ([report](../../../docs/test-reports.md)).
A second card (SDHC, 30475 MiB) in high speed mode; `bcm2835-sdhcid` not
started; root mounted from `0p1`; 8 MiB written to `/home/root` (3.7 MB/s
from `/scheme/rand`), the same SHA-256 after a restart; 100 program starts,
none failed; network and timesync (saved time, then NTP) work.

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
