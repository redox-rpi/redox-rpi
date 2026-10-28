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

## Coordination

The maintainer's coordinator session watches the queue every 10 minutes
and:

- reports a test that is marked **on the Pi** while the serial console and
  its owner have been silent for more than 15 minutes (a hung Pi);
- tells the maintainer early, in one sentence, when the next test needs a
  person at the Pi (card, power, keyboard, cables), and bundles such steps;
- decides the order when the Pi becomes free: tests for the current
  milestone ([v0.3.0](https://github.com/redox-rpi/redox-rpi/milestone/1))
  first, then by queue number; it names the next owner.

The rules above stay as they are: each owner takes and returns the Pi
itself.

## Serial console

The Pi's serial console is on `/dev/ttyUSB0` of the build host (115200
baud). A systemd user service, `redox-serial.service`, records everything
it prints to `~/redox-rpi/logs/serial.log` and restarts by itself.

- **Read** from that file (`tail -f`, `tail -c`), never from the device: a
  second reader on `/dev/ttyUSB0` takes bytes away from the log.
- **Type** with `printf '...\r' > /dev/ttyUSB0`, but only after the new
  boot's `redox login:` has appeared. Keys sent earlier reach the Redox
  bootloader ('e' opens its environment editor).
- **Old output:** `redox-serial-rotate.timer` moves the log every night at
  03:17 UTC to `~/redox-rpi/logs/serial/serial-<date>.log.gz` and deletes
  archives older than 14 days. Copy what a report needs into the report.

## Stable image

<!-- stable-image: written by scripts/update-stable.sh -->
Staged on the TFTP server whenever no test runs:
`v0.3.0-dev-15-g24fbb96+x9f76` (`main` 24fbb96, `--experiment timesync
--experiment rtl8152`, SD card through `bcm2835-sdhcid`), tested 2026-10-28
with `scripts/pi-test.py`: all checks pass, 0 of 300 starts failed
([report](test-reports.md)). A copy is in
`/srv/tftp/backup-v0.3.0-dev-15-g24fbb96+x9f76/`, and `/srv/tftp/stable`
points to it.
<!-- /stable-image -->

To restore the stable image after a test:

```bash
cp -p /srv/tftp/stable/* /srv/tftp/redox/
```

Older stable images stay in `/srv/tftp/backup-<build ID>/`, for example
`v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver` (until
2026-10-28, still with the T12 start failures).

## Queue

| # | Test | Owner | Image | Needs | Status |
|---|---|---|---|---|---|
| 16 | First run of the nightly stable image by hand: `scripts/update-stable.sh --force --ref origin/nightly-stable` builds, stages, resets the Pi, runs `scripts/pi-test.py` and promotes or restores ([branch](https://github.com/redox-rpi/redox-rpi/tree/nightly-stable)) | build agent | branch `nightly-stable`, `config/stable-image.conf` | network, serial console | waiting for the user's approval in the build agent's session |
| 4 | RTL8152/RTL8153 Ethernet on the Pi 3B+, with the Anker 565 hub (RTL8153B, works in QEMU with the real chip) or an RTL8152 adapter ([steps](https://github.com/redox-rpi/redox-rpi/tree/rtl8152/patches/experiments/rtl8152#a-pi-3b-with-a-usb-rtl8152-or-rtl8153-adapter)) | rtl8152 session | branch `rtl8152`, `--experiment ldso-diagnostics --experiment rtl8152` | person to plug the hub into the Pi (USB-C to USB-A adapter) and a LAN cable | waiting |
| 5 | dwc2d with interrupts: `scripts/pi-test.py` standard checks plus `ps` (CPU time of `dwc2d` near 0 instead of a full core), devsh traffic over the LAN7800, typing on a USB keyboard ([experiment](https://github.com/redox-rpi/redox-rpi/tree/dwc2-interrupts/patches/experiments/dwc2-interrupts)) | dwc2-interrupts session | branch `dwc2-interrupts`, `--experiment ldso-diagnostics --experiment dwc2-interrupts` | network; a person for the keyboard check | waiting |
| 17 | #1 start crash fix: relibc built with x18 reserved (the aarch64 signal entry returns with `br x18`); `scripts/pi-test.py --starts 1000`, the v0.3.0 goal is 1000 starts without a failure (stable image: 0 of 300; QEMU with the fix: 0 of 700, before 8 of 700) ([experiment](https://github.com/redox-rpi/redox-rpi/tree/t1-data-abort/patches/experiments/sigentry-x18), [#1](https://github.com/redox-rpi/redox-rpi/issues/1)) | t1 session | branch `t1-data-abort`, `--experiment timesync --experiment rtl8152 --experiment sigentry-x18` (the stable image plus the fix) | network, serial console | waiting, after #13 |
| 14 | T18 clean boot: the boot log on the serial console has no `pcid` panic, no `pcid-spawner` and no `audiod` errors; network up (DHCP address, devshd), typing on the USB keyboard works, `scripts/pi-test.py` standard checks as before (QEMU: 40 of 40 starts, [report](test-reports.md)) ([experiment](https://github.com/redox-rpi/redox-rpi/tree/pi-clean-boot/patches/experiments/pi-clean-boot)) | clean-boot session | branch `pi-clean-boot`, `--experiment ldso-diagnostics --experiment pi-clean-boot`; built as `v0.2.0-14-g5978d1a+x5b60` in `~/redox-rpi-t18` | network, serial console; a person for the keyboard check | waiting |
| 18 | v0.4.0, #15 direct firmware boot without U-Boot: flash `scripts/make-sdcard.sh --direct <harddrive.img>` (firmware, `kernel8.img` = `tools/rpi-boot-stub`, initramfs); the serial log shows the `rpi-boot-stub:` lines, the kernel start and `redox login:`; the `dwc2d` line has the board MAC `b8:27:eb:…`, not the fallback `02:52:65:64:6f:78`; `scripts/pi-test.py` standard checks; `/scheme/data` from the card. Afterwards the netboot card goes back in | boot session (#15) | branch `fw-direct-boot`, `make-sdcard.sh --direct` | serial console (read `~/redox-rpi/logs/serial.log`, no typing before `redox login:`); the user to swap the SD card | waiting, after all v0.3.0 tests |

## Done

| Date | Test | Build ID | Result |
|---|---|---|---|
| 2026-10-28 | T21 CPU clock and throttle watch (#13, [#20](https://github.com/redox-rpi/redox-rpi/issues/20)) | `v0.3.0-dev-31-g115d7fe+x572d` | [report](test-reports.md): 600 -> 1400 MHz works, starts 2x faster, standard checks pass, throttle warnings work; but under load at 1400 MHz 3–5 % of starts crash (600 MHz: 0.1 %), not ready as default |
| 2026-10-28 | SD card stress test (#15, [#6](https://github.com/redox-rpi/redox-rpi/issues/6)): 256 MiB, 1000 small files, 5 restarts, every file checked | `v0.3.0-dev-15-g24fbb96+x9f76` (sdhcid), `v0.3.0-dev-15-g24fbb96+x6d3c` (sdhost) | [report](test-reports.md): both pass, 0 missing, 0 changed; 256 MiB at 3.2 MB/s (sdhcid) and 5.7 MB/s (sdhost); a first run on alpha-28 discarded |
| 2026-10-28 | T12 fix (#11), kernel experiment [`sigdelivery-x0`](https://github.com/redox-rpi/redox-rpi/tree/t12-auxv-dump/patches/experiments/sigdelivery-x0) | `v0.2.0-9-g43f697e+xe0ce`, then `v0.2.0-9-g43f697e+x0f50` (without `ldso-diagnostics`) | [report](test-reports.md): `ps-tty` passes for the first time, `ps` 80 of 80, `reboot` 5 of 5; without `ldso-diagnostics` all checks pass and 1 of 900 starts fails (T1, data abort). Stable image restored |
| 2026-10-28 | Clock from NTP and saved time (#3), covered by the release tests #9 and #12 | `v0.1.0-alpha-76-g4178ec5+x83a0`, v0.3.0-rc1 | [report](test-reports.md): NTP time matches the build host, `/scheme/data/timesync.last` written; on the v0.3.0-rc1 SD card the boot set the build time, then the saved time, then pool.ntp.org |
| 2026-10-28 | Release candidate v0.2.0 on the Pi 3B+ and Zero 2 W (#9), released as v0.2.0 | `v0.1.0-alpha-76-g4178ec5+x83a0` (rc2) | [report](test-reports.md): 0 of 300 start failures, rpi-firmware, timesync and USB keyboard pass, `ps-tty` fails (T12). rc1 with `sdhost-driver`: sdhost by hand hung, so rc2 leaves it out |
| 2026-10-27 | Standard checks, diagnostics image | `v0.1.0-alpha-14-gf39d20b+ldso-diagnostics` | [report](test-reports.md): all pass except `ps-tty` |
| 2026-10-27 | Standard checks, default image | `v0.1.0-alpha-14-gf39d20b` | [report](test-reports.md): 36 of 300 start failures |
| 2026-10-27 | sdhost driver started by hand, read test | `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver` | [works](../patches/experiments/sdhost-driver/README.md#results): SDHC 15193 MiB, 4-bit, 25 MHz, partition table and FAT read correctly, 4.5 MB/s |
| 2026-10-27 | Firmware properties (`/scheme/rpi-firmware`), standard checks | `v0.1.0-alpha-51-g3d7e351+ldso-diagnostics+rpi-firmware` | [report](test-reports.md): all four files answer (Pi 3B+, 58.5 °C, core 250 MHz); standard checks pass except `ps-tty` |
| 2026-10-27 | sdhost driver at boot, write test across drivers | `v0.1.0-alpha-52-ga4ba2e3+ldso-diagnostics+sdhost-driver+sdhost-boot` | [works](test-reports.md): boots, 4 MiB written through sdhost read back by `bcm2835-sdhcid` with the same hash; writing 127 kB/s |
| 2026-10-27 | sdhost high speed mode, write speed with statistics | `v0.1.0-alpha-61-g90ef0ee+ldso-diagnostics+sdhost-driver+sdhost-boot+sdhost-stats` | [works](../patches/experiments/sdhost-driver/README.md#results): writing 1 MiB calls 6.7 MB/s, reading 13 MB/s; small writes slow because of RedoxFS (T16) |
| 2026-10-27 | Speed of `bcm2835-sdhcid` for comparison | `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver` | [report](test-reports.md): writing 2.7 MB/s (1 MiB calls), reading 4.5 MB/s; sdhost in high speed mode is 2.5x and 2.9x faster |
| 2026-10-28 | Network boot without SD card (#10): Pi firmware and U-Boot over TFTP, proxy DHCP next to the FritzBox | `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver` | [works](test-reports.md): boots to login without card; with the netboot card back it boots from the card as before |
| 2026-10-28 | v0.3.0-rc1: sdhost as the boot driver, root filesystem on the SD card (#12) | `v0.2.0-5-gd679bfb+xed14` | [works](test-reports.md): boots twice, 8 MiB written to the root filesystem with the same hash after a restart, 100 of 100 starts, no `bcm2835-sdhcid` |
