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

Experiment patches are numbered by what they apply after; see
[patches/README.md](../patches/README.md) for the numbers and the
combinations that build (for example `rtl8152` + `pi-clean-boot` +
`dwc2-interrupts`, in this order). Each applies on top of the default
patches.

## Coordination

The maintainer's coordinator session watches the queue every 10 minutes
and:

- reports a test that is marked **on the Pi** while the serial console and
  its owner have been silent for more than 15 minutes (a hung Pi);
- tells the maintainer early, in one sentence, when the next test needs a
  person at the Pi (card, power, keyboard, cables), and bundles such steps;
- has every test that needs a person prepared before the person comes:
  images built and their build IDs in the queue, so the person never waits
  for a build; the steps for one visit are bundled into as few boots as
  possible;
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
| 16 | First run of the nightly stable image by hand: `scripts/update-stable.sh --force --ref origin/nightly-stable` builds, stages, resets the Pi, runs `scripts/pi-test.py` and promotes or restores ([branch](https://github.com/redox-rpi/redox-rpi/tree/nightly-stable)) | build agent | branch `nightly-stable`, `config/stable-image.conf` | network, serial console | waiting: blocked by the agent's permission settings |
| 4 | RTL8152/RTL8153 Ethernet on the Pi 3B+, with the Anker 565 hub (RTL8153B, works in QEMU with the real chip) or an RTL8152 adapter ([steps](https://github.com/redox-rpi/redox-rpi/tree/rtl8152/patches/experiments/rtl8152#a-pi-3b-with-a-usb-rtl8152-or-rtl8153-adapter)) | rtl8152 session | branch `rtl8152`, `--experiment ldso-diagnostics --experiment rtl8152` | person to plug the hub into the Pi (USB-C to USB-A adapter) and a LAN cable | waiting |
| 22 | Serial console input ([#28](https://github.com/redox-rpi/redox-rpi/issues/28)): boot, log in as root on the serial console, then `patches/experiments/serial-rx/serial-input-test.sh 20` (writes 20 lines of 200 characters at full speed to `/dev/ttyUSB0`, checks the output in the serial log; done when 20 of 20 are complete); `scripts/pi-test.py` standard checks ([experiment](https://github.com/redox-rpi/redox-rpi/tree/serial-rx/patches/experiments/serial-rx)) | serial-rx session | branch `serial-rx` (merged with `main` b1fcbe1), `--experiment timesync --experiment rtl8152 --experiment serial-rx`, built as `v0.3.0-dev-94-gb1fcbe1+xe9c4` (QEMU 5 of 5); image and manifest in `~/t18-work/image-v0.3.0-dev-94-gb1fcbe1+xe9c4/` | network, serial console (writes to it) | waiting |
| 18 | v0.4.0, #15 direct firmware boot without U-Boot: flash `scripts/make-sdcard.sh --direct <harddrive.img>` (firmware, `kernel8.img` = `tools/rpi-boot-stub`, initramfs); the serial log shows the `rpi-boot-stub:` lines, the kernel start and `redox login:`; the `dwc2d` line has the board MAC `b8:27:eb:…`, not the fallback `02:52:65:64:6f:78`; `scripts/pi-test.py` standard checks; `/scheme/data` from the card. Afterwards the netboot card goes back in | boot session (#15) | branch `fw-direct-boot`, `make-sdcard.sh --direct` | serial console (read `~/redox-rpi/logs/serial.log`, no typing before `redox login:`); the user to swap the SD card | waiting, after all v0.3.0 tests |
| 21 | [#11](https://github.com/redox-rpi/redox-rpi/issues/11) devshd hangs up sessions whose client went away: `scripts/pi-test.py` standard checks, then 20 devsh sessions, 10 of them closed without `exit` (half with `sleep 600` running); `ps | cat` shows no extra `login`, `ion` or `sleep`, and new logins still work (QEMU: 6 dropped sessions leave nothing, [branch](https://github.com/redox-rpi/redox-rpi/tree/devshd-hangup)) | t1 session | branch `devshd-hangup`, `--experiment timesync --experiment rtl8152`, built as `v0.3.0-dev-77-g6bcd0fb+x9f76` | network | **on the Pi** (2026-10-28) |
| 23 | sdhost as the default SD driver ([#6](https://github.com/redox-rpi/redox-rpi/issues/6)), network boot: boot log shows `rpi-firmwared` before `bcm2835-sdhostd: core clock … MHz, up to … MHz`, `high speed`, `serving /scheme/disk.mmc`, and no `bcm2835-sdhcid`; `/scheme/data` mounted; `scripts/pi-test.py` standard checks; `scripts/sd-stress.py --cycles 2` | sdhost session | `v0.3.0-dev-76-g13092c5+x9f76` (branch `sdhost-default`, `--experiment timesync --experiment rtl8152`), built in `~/redox-rpi-sd`, staged by the owner | network, serial console | waiting for the Pi |
| 24 | sdhost as the default SD driver ([#6](https://github.com/redox-rpi/redox-rpi/issues/6)), root on the card, **Pi 3B+ and Zero 2 W**. For the person: write `http://homenodellm:8080/sdcard/redox-rpi-sdhost-default-v0.3.0-dev-76-g13092c5.img.xz` to a **spare** card (not the netboot card), boot the Pi 3B+ with it (HDMI, serial console; Enter at the resolution menu), then move the card to the Zero 2 W and boot it there. On each, report the last screen lines if it stops before `redox login:`. The owner then checks over the serial console: boot log `bcm2835-sdhostd: … high speed`, `cat /etc/redox-rpi-release`, 8 MiB `dd … conv=fsync` to `/home/root`, same SHA-256 after `echo reset > /scheme/sys/kstop`, 100 program starts | sdhost session | `v0.3.0-dev-76-g13092c5+x9f76` SD card image (`--sdcard`, `make-sdcard.sh --local`), ready | person to swap the card, spare SD card, Zero 2 W with serial console | ready, waiting for the person |
| 25 | v0.4.0, [#23](https://github.com/redox-rpi/redox-rpi/issues/23) CPU clock follows the load: boot log `ARM clock 600 -> 1400 MHz`; `scripts/pi-test.py` standard checks; `patches/experiments/cpu-ondemand/ondemand-check.py <repo> <pi> 2 30` (idle gives `target min` and 600 MHz, 50 starts timed in ondemand, performance and powersave, then 30 min of start rounds with 3 s pauses: no failures, switches grow) ([experiment](https://github.com/redox-rpi/redox-rpi/tree/cpu-ondemand/patches/experiments/cpu-ondemand)) | firmware session | branch `cpu-ondemand`, `--experiment timesync --experiment rtl8152 --experiment cpu-ondemand`, built as `v0.3.0-dev-93-g2d1a2e4+xc99c` (homenodellm `~/redox-rpi-cpu/keep/`) | network | waiting, after the v0.3.0 tests |
| 26 | v0.5.0, [#7](https://github.com/redox-rpi/redox-rpi/issues/7) poweroff: `scripts/pi-test.py` standard checks, then `poweroff` over devsh; the serial log ends with `kstop`, the green LED stays off, no ping answer for 2 minutes; then cut and reconnect the power, the Pi boots normally ([experiment](https://github.com/redox-rpi/redox-rpi/tree/poweroff/patches/experiments/poweroff)) | firmware session | branch `poweroff`, `--experiment timesync --experiment rtl8152 --experiment poweroff`, built as `v0.3.0-dev-95-gf6e2070+x7098` (homenodellm `~/redox-rpi-cpu/keep/`) | network; **a person to cut and reconnect the power** at the end | waiting, after the v0.3.0 tests |
| 27 | v0.4.0, [#22](https://github.com/redox-rpi/redox-rpi/issues/22) timerd, network boot: boot log shows `timerd: clock set by timesync` and the two test timers; after 3 minutes `timerd -l` and the log show `test-minute` every minute and `test-catchup` every 2 minutes, each `exited with code 0`; `cat /scheme/data/timerd/test-catchup`. Then `echo 1793000000 > /scheme/data/timerd/test-catchup` (an old last run) and `reboot`: the log shows `test-catchup: starting uname (missed since 2026-10-26 …)` once, `test-minute` does not catch up; `scripts/pi-test.py` standard checks ([experiment](https://github.com/redox-rpi/redox-rpi/tree/timerd/patches/experiments/timerd)) | RTC/Uhrzeit session | `v0.3.0-dev-93-g2143ad7+x1044` (branch `timerd`, `--experiment ldso-diagnostics --experiment timesync --experiment timerd --experiment timerd-test`), built in `~/redox-rpi-timerd`, image `~/timerd-images/netboot-harddrive.img`, staged by the owner | network, serial console | ready, waiting for the Pi |

## Done

| Date | Test | Build ID | Result |
|---|---|---|---|
| 2026-10-28 | #5 dwc2d with interrupts, with `pi-clean-boot` (#18), keyboard by the maintainer | `v0.3.0-dev-52-g7df70f9+xcb30` | [report](test-reports.md): all pass, 0 of 300 starts failed, `dwc2d` about 5 % of a core instead of a full one |
| 2026-10-28 | #20 CPU clock at 1400 MHz with the #1 x18 fix (#19) | `v0.3.0-dev-59-g1559bca+x9906` | [report](test-reports.md): **pass**, standard checks pass, 0 of 7000 starts failed in 30 minutes at 1400 MHz (without the fix, #13: 276 of 5100) |
| 2026-10-28 | #14: T18 clean boot ([#18](https://github.com/redox-rpi/redox-rpi/issues/18)), experiment `pi-clean-boot` on the stable base | `v0.3.0-dev-49-g832e1b5+x6256` | [works](https://github.com/redox-rpi/redox-rpi/blob/pi-clean-boot/docs/test-reports.md): no `pcid`, `pcid-spawner`, `audiod` or `rtcd` in the boot log, network up, `pi-test.py` all pass, 0 of 300 starts failed; USB keyboard typing still to check (user, with #5). Stable image back |
| 2026-10-28 | #1 start crash fix (#17): relibc built with x18 reserved, experiment [`sigentry-x18`](https://github.com/redox-rpi/redox-rpi/tree/t1-data-abort/patches/experiments/sigentry-x18), on top of the stable image's experiments, 600 MHz | `v0.3.0-dev-34-g04f0123+x4d10` | [report](test-reports.md): all checks pass, **0 of 1000 starts failed**, no exception on the serial console (stable image: 0 of 300, before the T12 fix 1 of 900). One chunk lost its devsh connection before starting anything; counted apart. Stable image restored |
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
