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

### Nightly update

**The Pi may reboot at night.** Every night at 03:30 UTC the build host runs
[`scripts/update-stable.sh`](../scripts/update-stable.sh) (systemd user
timer `redox-stable.timer`, [config/redox-stable.timer](../config/redox-stable.timer)):

1. It stops if `patches/`, `config/`, `scripts/` and `tools/` on
   `origin/main` are unchanged since the stable image's commit, or if a test
   in the queue is marked **on the Pi**. A test that runs over night must be
   marked so.
2. It builds `origin/main` in `~/redox-rpi-stable` with the options in
   [config/stable-image.conf](../config/stable-image.conf), stages it,
   resets the Pi and runs `scripts/pi-test.py`.
3. Pass: the image becomes the stable image. The script backs it up, updates
   the block above, appends the report to [test-reports.md](test-reports.md)
   and pushes both to `main`.
4. Fail: the previous stable image is staged again, and the issue "Nightly:
   main fails on the Pi" (`topic:ci`, `needs:pi`) gets the build ID and the
   failing checks.

Logs: `~/redox-rpi/logs/stable/<date>.log` on the build host. Run it by hand
by marking its queue entry N **on the Pi** and running
`scripts/update-stable.sh --force --taken N` (`--force`: build even without
changes; `--taken N`: entry N is this run, any other test on the Pi still
stops it; `--no-push`: no commit, no issue).

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
| 16 | First run of the nightly stable image by hand: `scripts/update-stable.sh --force --taken 16 --ref origin/nightly-stable` builds, stages, resets the Pi, runs `scripts/pi-test.py` and promotes or restores ([branch](https://github.com/redox-rpi/redox-rpi/tree/nightly-stable)) | build agent | branch `nightly-stable`, `config/stable-image.conf` | network, serial console | waiting: the first run failed, [#34](https://github.com/redox-rpi/redox-rpi/issues/34) (devsh hang, fixed on main); next run with the fixes on `nightly-stable` |
| 4 | RTL8152/RTL8153 Ethernet on the Pi 3B+, with the Anker 565 hub (RTL8153B, works in QEMU with the real chip) or an RTL8152 adapter ([steps](https://github.com/redox-rpi/redox-rpi/tree/rtl8152/patches/experiments/rtl8152#a-pi-3b-with-a-usb-rtl8152-or-rtl8153-adapter)) | rtl8152 session | branch `rtl8152`, `--experiment ldso-diagnostics --experiment rtl8152` | person to plug the hub into the Pi (USB-C to USB-A adapter) and a LAN cable | waiting |
| 33 | v0.3.0, clean boot as the default patch ([#18](https://github.com/redox-rpi/redox-rpi/issues/18), base `0024`, was experiment `pi-clean-boot`, passed as an experiment in #14 and #5), network boot: the boot log has no `pcid`, `pcid-spawner`, `audiod` or `rtcd` lines and no `pcid` panic; network up; `scripts/pi-test.py` standard checks. Short test | t18 session | `v0.3.0-dev-129-g6917241+x9f76` (branch [`clean-boot-default`](https://github.com/redox-rpi/redox-rpi/tree/clean-boot-default), `--experiment timesync --experiment rtl8152`, the stable image's options), homenodellm `~/t18-work/image-cbd/`, staged by the owner; checked in QEMU | network, serial console (reads the log only) | waiting |
| 18 | v0.4.0, #15 direct firmware boot without U-Boot: flash the card (firmware, `kernel8.img` = `tools/rpi-boot-stub`, initramfs); the serial log shows the `rpi-boot-stub:` lines (with `framebuffer …` from #33), the kernel start and `redox login:`; the `dwc2d` line has the board MAC `b8:27:eb:…`, not the fallback `02:52:65:64:6f:78`; `scripts/pi-test.py` standard checks; `/scheme/data` from the card; `fatfsd /scheme/disk.mmc/0p0 boot &` then `ls /scheme/boot` shows `config.txt` (redox-fatfs2); HDMI shows the console if a screen is attached. Afterwards the netboot card goes back in | boot session (#15) | `http://homenodellm:8080/sdcard/redox-rpi-t18-direct-v0.3.0-dev-106-g3b79cd4+x16ea.img.xz` (branch `fw-direct-boot` 55e567a, `--experiment timesync --experiment rtl8152 --experiment redox-fatfs2`, checked in QEMU) | serial console (read `~/redox-rpi/logs/serial.log`, no typing before `redox login:`); the user to swap the SD card | waiting, after all v0.3.0 tests; image ready |
| 23 | sdhost as the default SD driver ([#6](https://github.com/redox-rpi/redox-rpi/issues/6)), network boot: boot log shows `rpi-firmwared` before `bcm2835-sdhostd: core clock … MHz, up to … MHz`, `high speed`, `serving /scheme/disk.mmc`, and no `bcm2835-sdhcid`; `/scheme/data` mounted; `scripts/pi-test.py` standard checks; `scripts/sd-stress.py --cycles 2` | sdhost session | `v0.3.0-dev-76-g13092c5+x9f76` (branch `sdhost-default`, `--experiment timesync --experiment rtl8152`), built in `~/redox-rpi-sd`, staged by the owner | network, serial console | **failed**, Pi halted (kernel `oom` panic during `sd-stress.py`), needs a power cycle; stable image staged again |
| 30 | Rerun of #23 with the kernel memory fix: sdhost as the default SD driver ([#6](https://github.com/redox-rpi/redox-rpi/issues/6)) and threads that end themselves no longer keep their kernel stack ([#31](https://github.com/redox-rpi/redox-rpi/issues/31)), network boot. Same checks as #23: boot log shows `rpi-firmwared` before `bcm2835-sdhostd: core clock … MHz, up to … MHz`, `high speed`, `serving /scheme/disk.mmc`, and no `bcm2835-sdhcid`; `/scheme/data` mounted; `scripts/pi-test.py` standard checks; `scripts/sd-stress.py --cycles 2`, with `free` before and after (QEMU: no growth over 600 `dd` runs) ([experiment](https://github.com/redox-rpi/redox-rpi/tree/sdhost-ctxleak/patches/experiments/ctx-leak-fix)) | sdhost session | `v0.3.0-dev-120-g5ffb097+x197a` (branch `sdhost-ctxleak` = `sdhost-default` + main with the devshd hangup fix [#11](https://github.com/redox-rpi/redox-rpi/issues/11) + `--experiment ctx-leak-fix`, with `--experiment timesync --experiment rtl8152`), homenodellm `~/redox-rpi-sd/keep/`, staged by the owner. Fallback if it fails: `v0.3.0-dev-110-gefa605d+x197a` (stable base + fix, branch `ctx-leak`, same folder) | network, serial console; the Pi needs a power cycle first (halted after #23) | ready for the rerun with `v0.3.0-dev-120-g5ffb097+x197a` (first run with `v0.3.0-dev-80-g8bba533+x197a` stopped: devshd hang [#11](https://github.com/redox-rpi/redox-rpi/issues/11) after restart cycle 1; phase 1 including the 1000 small files passed, no oom, [report](test-reports.md)) |
| 24 | sdhost as the default SD driver ([#6](https://github.com/redox-rpi/redox-rpi/issues/6)), root on the card, **Pi 3B+ and Zero 2 W**. For the person: write `http://homenodellm:8080/sdcard/redox-rpi-sdhost-default-v0.3.0-dev-76-g13092c5.img.xz` to a **spare** card (not the netboot card), boot the Pi 3B+ with it (HDMI, serial console; Enter at the resolution menu), then move the card to the Zero 2 W and boot it there. On each, report the last screen lines if it stops before `redox login:`. The owner then checks over the serial console: boot log `bcm2835-sdhostd: … high speed`, `cat /etc/redox-rpi-release`, 8 MiB `dd … conv=fsync` to `/home/root`, same SHA-256 after `echo reset > /scheme/sys/kstop`, 100 program starts | sdhost session | `v0.3.0-dev-76-g13092c5+x9f76` SD card image (`--sdcard`, `make-sdcard.sh --local`), ready | person to swap the card, spare SD card, Zero 2 W with serial console | waits for a later release: no Zero 2 W on the test network. For v0.3.0, #6 is judged on the Pi 3B+ only (test 30) |
| 25 | v0.4.0, [#23](https://github.com/redox-rpi/redox-rpi/issues/23) CPU clock follows the load: boot log `ARM clock 600 -> 1400 MHz`; `scripts/pi-test.py` standard checks; `patches/experiments/cpu-ondemand/ondemand-check.py <repo> <pi> 2 30` (idle gives `target min` and 600 MHz, 50 starts timed in ondemand, performance and powersave, then 30 min of start rounds with 3 s pauses: no failures, switches grow) ([experiment](https://github.com/redox-rpi/redox-rpi/tree/cpu-ondemand/patches/experiments/cpu-ondemand)) | firmware session | branch `cpu-ondemand`, `--experiment timesync --experiment rtl8152 --experiment cpu-ondemand`, built as `v0.3.0-dev-93-g2d1a2e4+xc99c` (homenodellm `~/redox-rpi-cpu/keep/`) | network | waiting, after the v0.3.0 tests |
| 26 | v0.5.0, [#7](https://github.com/redox-rpi/redox-rpi/issues/7) poweroff: `scripts/pi-test.py` standard checks, then `poweroff` over devsh; the serial log ends with `kstop`, the green LED stays off, no ping answer for 2 minutes; then cut and reconnect the power, the Pi boots normally ([experiment](https://github.com/redox-rpi/redox-rpi/tree/poweroff/patches/experiments/poweroff)) | firmware session | branch `poweroff`, `--experiment timesync --experiment rtl8152 --experiment poweroff`, built as `v0.3.0-dev-95-gf6e2070+x7098` (homenodellm `~/redox-rpi-cpu/keep/`) | network; **a person to cut and reconnect the power** at the end | waiting, after the v0.3.0 tests |
| 27 | v0.4.0, [#22](https://github.com/redox-rpi/redox-rpi/issues/22) timerd, network boot: boot log shows `timerd: clock set by timesync` and the two test timers; after 3 minutes `timerd -l` and the log show `test-minute` every minute and `test-catchup` every 2 minutes, each `exited with code 0`; `cat /scheme/data/timerd/test-catchup`. Then `echo 1793000000 > /scheme/data/timerd/test-catchup` (an old last run) and `reboot`: the log shows `test-catchup: starting uname (missed since 2026-10-26 …)` once, `test-minute` does not catch up; `scripts/pi-test.py` standard checks ([experiment](https://github.com/redox-rpi/redox-rpi/tree/timerd/patches/experiments/timerd)) | RTC/Uhrzeit session | `v0.3.0-dev-93-g2143ad7+x1044` (branch `timerd`, `--experiment ldso-diagnostics --experiment timesync --experiment timerd --experiment timerd-test`), built in `~/redox-rpi-timerd`, image `~/timerd-images/netboot-harddrive.img`, staged by the owner | network, serial console | **on the Pi** (2026-10-29) |
| 28 | v0.4.0, [#3](https://github.com/redox-rpi/redox-rpi/issues/3) cache maintenance when user code is loaded (kernel: clean to PoU and `ic ialluis` on `mprotect` to executable and at exec): `scripts/pi-test.py --starts 1000` at 600 MHz, and the start loop's time against the stable image (QEMU: 0 of 700, loop 6-8 % slower) ([experiment](https://github.com/redox-rpi/redox-rpi/tree/t3-icache/patches/experiments/icache-sync)) | t1 session | branch `t3-icache`, `--experiment timesync --experiment rtl8152 --experiment icache-sync`, built as `v0.3.0-dev-104-g3f88563+x508a` | network | waiting |
| 29 | v0.4.0, [#24](https://github.com/redox-rpi/redox-rpi/issues/24) small SD card image that grows on first boot, Pi 3B+. For the person: write `http://homenodellm:8080/sdcard/redox-rpi-grow-root-v0.3.0-dev-81-g5ceb150.img.xz` (149 MiB) to a **spare** card of 8 GB or more, boot the Pi 3B+ with it (serial console). The owner checks: first boot logs `growfs: partition 2 from 85 MiB to … MiB (the whole card), then restart`, the second boot `growfs: RedoxFS on /scheme/disk.mmc/0p1 from 84 MiB to … MiB`, `df -h` shows the card's size, a third boot logs nothing from `growfs`; `scripts/pi-test.py` standard checks ([experiment](https://github.com/redox-rpi/redox-rpi/tree/grow-root/patches/experiments/grow-root)) | sdhost session | branch `grow-root` (on `sdhost-default`), `make-sdcard.sh --small`, built as `v0.3.0-dev-81-g5ceb150` | person to swap the card, spare SD card of 8 GB or more | waiting, after #23 and #24 (needs the `sdhost-default` fix) |
| 31 | [#19](https://github.com/redox-rpi/redox-rpi/issues/19) fast network boot on the **Zero 2 W** (does not use the Pi 3B+ or the shared TFTP directories). For the person: write `http://homenodellm:8080/sdcard/redox-rpi-zero2w-fastnet-db5ea27.img.xz` (2 MB, 32 MiB card image, any card of 64 MB or more, may replace the old Zero netboot card) to a card, put it in the Zero with the Ethernet/USB HAT and a LAN cable, power on, and time it from power on to `redox login:`. Note or photograph the screen: U-Boot's `wget` of `harddrive.img.gz` (42 MB) should finish in seconds with no `Rx: failed to receive`; if HTTP fails, the script falls back to TFTP with full blocks, note that too. In Redox: `dwc2d: RTL8152 chip version …`, `link up`, `network adapter <MAC>`. The owner then pings the Zero and runs `scripts/pi-test.py <address>` from the build host | rtl8152 session | U-Boot v2026.10 + `patches/u-boot/0001` (16 KiB r8152 receive buffer, branch [`zero2w-fastnet`](https://github.com/redox-rpi/redox-rpi/tree/zero2w-fastnet) db5ea27); Redox `v0.1.0-alpha-90-g7fa2379+x332f` (`--zero2w --experiment rtl8152`), staged in `/srv/tftp/redox-zero2w-fast/` over HTTP | person with the Zero 2 W, HAT, LAN cable and a screen (or serial console) | waiting: no Zero 2 W on the network, later release; not checked in QEMU (no RTL815x adapter on the build host now) |
| 32 | v0.7.0, [#35](https://github.com/redox-rpi/redox-rpi/issues/35) `lsusb` and `hwinfo`: `scripts/pi-test.py` standard checks; `lsusb`, `lsusb -t`, `lsusb -v` show the 3B+ hubs `0424:2514` and the LAN7800 `0424:7800`; `hwinfo` shows Pi 3 Model B+, 1 GiB, 4 CPUs, clocks and temperature matching `/scheme/rpi-firmware`. Optional, with a person: plug a keyboard in and out, `lsusb` follows (QEMU passes `patches/experiments/hwinfo/qemu-check.sh`, [experiment](https://github.com/redox-rpi/redox-rpi/tree/hwinfo/patches/experiments/hwinfo)) | firmware session | branch `hwinfo` (on top of `usb-hotplug`), `--experiment timesync --experiment usb-hotplug --experiment hwinfo`, built as `v0.3.0-dev-149-ga3a4670+x02a9` (homenodellm `~/redox-rpi-cpu/keep/`) | network; a person only for the optional plug test | waiting, after the `usb-hotplug` Pi test, low priority (v0.7.0) |
| 34 | No release, [#33](https://github.com/redox-rpi/redox-rpi/issues/33) display through the firmware (`rpi-fbd`), network boot. Boot log: `rpi-fbd: EDID … bytes, monitor …` (or `no EDID`), `rpi-fbd: boot framebuffer 1920x1200 stride … at …`, then `fbbootlogd: handoff requested` and `fbcond: Performing handoff`, no `rpi-fbd` panic; `cat /scheme/rpi-firmware/board` still answers; `scripts/pi-test.py` standard checks. Optional second boot, only if the coordinator allows typing at the bootloader menu: `e`, type `RPI_FB=1280x720`, Enter twice; log `rpi-fbd: 1280x720 stride … at …`. For a person, only together with another visit (no trip of its own): the screen shows the console as before; on the second boot it is 1280x720 scaled to the full screen; INFO in blue, not red; and one boot with the HDMI cable out from power-on, reaching `redox login:` on serial with `rpi-fbd: no EDID`; then the monitor-off check for `hdmi_force_hotplug` (user finding 2026-10-29: monitor off at power-on, turned on later, no picture): the person adds the line `hdmi_force_hotplug=1` at the end of `config.txt` on the netboot card's FAT partition (as on branch `graphics-fb`, `config/config.txt`), boots with the monitor switched off, turns it on after `redox login:` and says whether the console shows and at what size (serial log: `vesad: …x…`); the line stays on the card (if the Pi boots without card, the owner puts the branch's `config.txt` into `/srv/tftp/` for the test and restores it after) ([experiment](https://github.com/redox-rpi/redox-rpi/tree/graphics-fb/patches/experiments/rpi-fb)) | graphics session | `v0.3.0-dev-132-gae30bad+xb2e2` (branch `graphics-fb`, `--experiment timesync --experiment rtl8152 --experiment rpi-fb`), homenodellm `~/redox-rpi-graphics/keep/`, staged by the owner | network, serial console; the screen checks need a person (optional, bundle) | waiting, after the v0.3.0 tests |
| 35 | v0.3.0, [#17](https://github.com/redox-rpi/redox-rpi/issues/17) USB hotplug and `dwc2-default` (RTL8152 + dwc2 interrupts as default patches, adapter choice by link). **Part A, no person:** `scripts/pi-test.py` standard checks (this also covers `dwc2-default`); boot log `dwc2d: hub at address … power switching per port\|ganged` (does the LAN7515 switch ports one by one?); `ps` CPU time of `dwc2d` (test #5: about 5 % of a core); then `scripts/pi-hotplug-test.sh 192.168.178.148 3`: dwc2d switches the LAN7800's hub port off and on three times through `/tmp/dwc2d-cycle`, the log shows `removed`, `LAN7800 at address`, `network on the adapter`, and devshd answers again each time. **Part B, with the person (keyboard only):** (1) unplug the USB keyboard, wait a few seconds, plug it back in and type `ls` on the HDMI console; repeat a few times, on different USB ports of the Pi: log `keyboard removed`, `keyboard at address …`; (2) power cycle with the LAN cable in the Pi, pull the cable when the Redox boot log starts and put it back after `redox login:`: `dhcpd: … trying again`, then `dhcpd: configured after N attempts`, devshd answers. The Anker hub steps stay with test #4 for later. QEMU (`scripts/test-hotplug.sh`): 16 of 16 pass ([experiment](https://github.com/redox-rpi/redox-rpi/tree/usb-hotplug/patches/experiments/usb-hotplug)) | usb session | `v0.3.0-dev-148-g54f7433+x657a` (branch `usb-hotplug` = `dwc2-default` + main + `--experiment usb-hotplug`), homenodellm `~/redox-rpi-usb/keep/`, staged by the owner | part A: network, serial console; part B: a person with a USB keyboard, the LAN cable and the HDMI screen | next, right after #27 (person at the Pi); image ready |
| 36 | [#1](https://github.com/redox-rpi/redox-rpi/issues/1) complete x18 fix: the signal entry returns through `svc #0x5352` and clobbers no register (kernel and relibc, [experiment](https://github.com/redox-rpi/redox-rpi/tree/t1-sigreturn/patches/experiments/sigreturn-svc)): `sigregs process -n 3000` and `sigregs thread -n 3000` (now with x18) pass, `scripts/pi-test.py --starts 1000` at 600 MHz, no exception on the serial console (QEMU: both pass, 0 of 700; without the fix x18 changes after 2-3 signals) | t1 session | branch `t1-sigreturn`, `--experiment timesync --experiment rtl8152 --experiment sigreturn-svc --experiment sigregs-test`, built as `v0.3.0-dev-139-g66f7cba+x0002` (homenodellm `~/redox-rpi-t12/build/sigreturn-pi.img`) | network, serial console | waiting |
| 37 | [#4](https://github.com/redox-rpi/redox-rpi/issues/4) thread signals, review patch `0002` (+64 for signals sent to a thread, sender of process signals, [experiment](https://github.com/redox-rpi/redox-rpi/tree/t4-thread-signals/patches/experiments/sigentry-thread-signals)): `sigregs sender`, `sigregs thread -n 3000`, `sigregs process -n 3000` pass; `scripts/pi-test.py` standard checks (QEMU: all pass; without `0002` `sender` shows the wrong si_pid) | t1 session | branch `t4-thread-signals`, `--experiment timesync --experiment rtl8152 --experiment sigentry-thread-signals --experiment sigregs-test`, built as `v0.3.0-dev-140-g55bd363+x68b9` (homenodellm `~/redox-rpi-t12/build/t4-pi.img`) | network | waiting |
| 38 | v0.8.0, [#38](https://github.com/redox-rpi/redox-rpi/issues/38) GPIO driver `bcm2835-gpiod` and the 3B+ power LED: `scripts/pi-test.py` standard checks; green LED on and off through `/scheme/gpio/29` (`echo out > mode`, `echo 1 > value`, `echo 0 > value`), red LED off and on through `/scheme/rpi-firmware/power_led`; GPIO 27 `in` reads 1 with `pull` `up` and 0 with `down`; optional jumper pin 11 to pin 13 (GPIO 17 out, 27 in) ([steps](https://github.com/redox-rpi/redox-rpi/tree/gpio/patches/experiments/gpio#test-on-the-pi); QEMU passes `qemu-check.sh`) | firmware session | branch `i2c` (contains `gpio`), `--experiment timesync --experiment rtl8152 --experiment gpio --experiment i2c`, built as `v0.3.0-dev-146-g5a7fc2c+x630f` (homenodellm `~/redox-rpi-cpu/keep/`), the same image for #38 and #39 | network; a person to watch the LEDs (jumper optional) | waiting, low priority (v0.8.0) |
| 39 | v0.8.0, [#39](https://github.com/redox-rpi/redox-rpi/issues/39) I2C bus 1 driver `bcm2835-i2cd` and the `i2c` tool with a DS3231 clock module: boot log `bcm2835-i2cd: bus 1 at 0x3f804000, 100 kHz`; `i2c detect` shows 68 (57 with an EEPROM); `i2c get 68 0 7` time in BCD, seconds go on; `i2c get 68 11 2` temperature; write and read back `i2c set 68 7 12`; without the module `i2c detect` is empty and does not hang ([steps and wiring](https://github.com/redox-rpi/redox-rpi/tree/i2c/patches/experiments/i2c#test-on-the-pi); QEMU passes `qemu-check.sh` with a DS1338) | firmware session | built as `v0.3.0-dev-146-g5a7fc2c+x630f` (homenodellm `~/redox-rpi-cpu/keep/`), the same image for #38 and #39 | network; a person to wire the DS3231 with power off: VCC pin 1 (3.3 V), SDA pin 3, SCL pin 5, GND pin 6 | waiting for the person, low priority (v0.8.0) |

## Done

| Date | Test | Build ID | Result |
|---|---|---|---|
| 2026-10-28 | #22: serial console input ([#28](https://github.com/redox-rpi/redox-rpi/issues/28)), experiment `serial-rx` on the stable base | `v0.3.0-dev-94-gb1fcbe1+xe9c4` | [works](https://github.com/redox-rpi/redox-rpi/blob/serial-rx/docs/test-reports.md): 20 of 20 lines of 200 characters complete at full speed; `pi-test.py` all checks pass, 0 of 300 starts failed (first attempt: no devsh answer, #11). Stable image back |
| 2026-10-28 | devshd hangs up sessions whose client went away (#21, [#11](https://github.com/redox-rpi/redox-rpi/issues/11), [branch](https://github.com/redox-rpi/redox-rpi/tree/devshd-hangup)) | `v0.3.0-dev-77-g6bcd0fb+x9f76` | [report](test-reports.md): **pass**, standard checks pass (0 of 300 starts failed); 10 clean and 13 dropped devsh sessions (5 with `sleep 600` running) leave no `login`, `ion` or `sleep` behind, devshd stays at 4 threads. Stable image restored |
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
