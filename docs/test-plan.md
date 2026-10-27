# Test plan for the real Pi

There is one Raspberry Pi 3B+ on the test network and one build host, and
several people or agent sessions want to use them. This page says who has
the Pi now and which test comes next. Only the test marked **on the Pi** may
stage files on the TFTP server (`scripts/netboot.sh`), reboot the Pi or run
tests on it.

## Rules

1. **Add your test to the queue** before you build for it: name, owner
   (person or session), the experiments, and what the test checks.
2. **Take the Pi** only when your test is first in the queue and the
   previous one is marked done. Set it to **on the Pi** and commit.
3. **Build:** `scripts/build-image.sh` waits for any other build on the
   same host to finish (lock file `build/.build-image.lock`). Run other jobs
   that use the build output, such as a QEMU test, under
   `scripts/with-build-lock.sh <command>`; it takes the same lock, and a
   `build-image.sh` inside it does not lock again. Never wrap a command in
   `flock` on that file yourself: `build-image.sh` would wait for it forever.
4. **Test:** stage the image, reboot the Pi, run the steps. Automated checks
   go through [`scripts/pi-test.py`](../scripts/pi-test.py).
5. **Give the Pi back:** put the stable image back on the TFTP server
   (see "Stable image" below), write the results into
   [test-reports.md](test-reports.md), mark the test **done** and move it to
   "Done".
6. **Merge only after the Pi test:** new features and experiments live on
   their own branch until this test passed in detail; only then do they go
   to `main` ([CONTRIBUTING.md](../CONTRIBUTING.md#branches)).

Tests that need a person at the Pi (keyboard, screen, power) say so; the
owner then asks that person before taking the Pi.

Experiments that add a driver to base each carry their own patch `0017`,
so `sdhost-driver` (with `sdhost-boot`), `rtl8152` and `dwc2-interrupts`
cannot be combined in one build yet. Each of them applies on top of the
default patches, which include `rpi-firmwared` (base `0018`) since
2026-10-27.

## Stable image

Staged on the TFTP server whenever no test runs:
`v0.1.0-alpha-14-gf39d20b+ldso-diagnostics` (0 of 300 start failures,
[report](test-reports.md)).

No copy of this image is left on the build host: staging another image
overwrote it. Until it is rebuilt, the TFTP server has
`v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver`, which starts the
same drivers (`bcm2835-sdhcid` for the card). Keep a copy of the stable
image next to the TFTP directory once it is rebuilt.

```bash
scripts/netboot.sh --server 192.168.178.120 --tftp-dir /srv/tftp --image <stable harddrive.img>
```

## Queue

| # | Test | Owner | Image | Needs | Status |
|---|---|---|---|---|---|
| 9 | Release candidate v0.2.0-alpha in one pass, replaces #3 and #4 on the Pi 3B+: `scripts/pi-test.py --starts 300`, timesync (`date`, `timesync -n`, saved time), the four `/scheme/rpi-firmware` files, sdhost started by hand (read and write), USB keyboard; then the Zero 2 W SD image with the RTL8152 hub | release session | branch `release-v0.2.0`, `--experiment ldso-diagnostics --experiment timesync --experiment sdhost-driver --experiment rtl8152` | network, serial console; the user for keyboard and Zero 2 W | waiting |
| 3 | Clock from NTP and saved time: `date` after boot, `timesync -n`, `cat /scheme/data/timesync.last`; reboot with the Ethernet cable out, `date` must not be before the last run ([experiment](../patches/experiments/timesync/README.md)) | timesync session | `--experiment ldso-diagnostics --experiment timesync` | network only | waiting |
| 4 | RTL8152/RTL8153 Ethernet on the Pi 3B+, with the Anker 565 hub (RTL8153B, works in QEMU with the real chip) or an RTL8152 adapter ([steps](https://github.com/redox-rpi/redox-rpi/tree/rtl8152/patches/experiments/rtl8152#a-pi-3b-with-a-usb-rtl8152-or-rtl8153-adapter)) | rtl8152 session | branch `rtl8152`, `--experiment ldso-diagnostics --experiment rtl8152` | person to plug the hub into the Pi (USB-C to USB-A adapter) and a LAN cable | waiting |
| 5 | dwc2d with interrupts: `scripts/pi-test.py` standard checks plus `ps` (CPU time of `dwc2d` near 0 instead of a full core), devsh traffic over the LAN7800, typing on a USB keyboard ([experiment](https://github.com/redox-rpi/redox-rpi/tree/dwc2-interrupts/patches/experiments/dwc2-interrupts)) | dwc2-interrupts session | branch `dwc2-interrupts`, `--experiment ldso-diagnostics --experiment dwc2-interrupts` | network; a person for the keyboard check | waiting |

## Done

| Date | Test | Build ID | Result |
|---|---|---|---|
| 2026-10-27 | Standard checks, diagnostics image | `v0.1.0-alpha-14-gf39d20b+ldso-diagnostics` | [report](test-reports.md): all pass except `ps-tty` |
| 2026-10-27 | Standard checks, default image | `v0.1.0-alpha-14-gf39d20b` | [report](test-reports.md): 36 of 300 start failures |
| 2026-10-27 | sdhost driver started by hand, read test | `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver` | [works](../patches/experiments/sdhost-driver/README.md#results): SDHC 15193 MiB, 4-bit, 25 MHz, partition table and FAT read correctly, 4.5 MB/s |
| 2026-10-27 | Firmware properties (`/scheme/rpi-firmware`), standard checks | `v0.1.0-alpha-51-g3d7e351+ldso-diagnostics+rpi-firmware` | [report](test-reports.md): all four files answer (Pi 3B+, 58.5 °C, core 250 MHz); standard checks pass except `ps-tty` |
| 2026-10-27 | sdhost driver at boot, write test across drivers | `v0.1.0-alpha-52-ga4ba2e3+ldso-diagnostics+sdhost-driver+sdhost-boot` | [works](test-reports.md): boots, 4 MiB written through sdhost read back by `bcm2835-sdhcid` with the same hash; writing 127 kB/s |
| 2026-10-27 | sdhost high speed mode, write speed with statistics | `v0.1.0-alpha-61-g90ef0ee+ldso-diagnostics+sdhost-driver+sdhost-boot+sdhost-stats` | [works](../patches/experiments/sdhost-driver/README.md#results): writing 1 MiB calls 6.7 MB/s, reading 13 MB/s; small writes slow because of RedoxFS (T16) |
| 2026-10-27 | Speed of `bcm2835-sdhcid` for comparison | `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver` | [report](test-reports.md): writing 2.7 MB/s (1 MiB calls), reading 4.5 MB/s; sdhost in high speed mode is 2.5x and 2.9x faster |
