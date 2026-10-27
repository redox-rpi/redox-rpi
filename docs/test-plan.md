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
   same host to finish (lock file `build/.build-image.lock`).
4. **Test:** stage the image, reboot the Pi, run the steps. Automated checks
   go through [`scripts/pi-test.py`](../scripts/pi-test.py).
5. **Give the Pi back:** put the stable image back on the TFTP server
   (see "Stable image" below), write the results into
   [test-reports.md](test-reports.md), mark the test **done** and move it to
   "Done".

Tests that need a person at the Pi (keyboard, screen, power) say so; the
owner then asks that person before taking the Pi.

Experiments that add a driver to base each carry their own patch `0017`,
so `sdhost-driver`, `rpi-firmware` and `rtl8152` cannot be combined in one
build yet.

## Stable image

Staged on the TFTP server whenever no test runs:
`v0.1.0-alpha-14-gf39d20b+ldso-diagnostics` (0 of 300 start failures,
[report](test-reports.md)).

```bash
scripts/netboot.sh --server 192.168.178.120 --tftp-dir /srv/tftp --image <stable harddrive.img>
```

## Queue

| # | Test | Owner | Image | Needs | Status |
|---|---|---|---|---|---|
| 1 | sdhost driver, started by hand next to `bcm2835-sdhcid` ([steps](../patches/experiments/sdhost-driver/README.md#manual-test-on-the-pi)) | Stephan, image from the sdhost session | `--experiment ldso-diagnostics --experiment sdhost-driver` | person at the console | next |
| 2 | Firmware properties: board, temperature, clocks (`/scheme/rpi-firmware`) | rpi-firmware session | `--experiment ldso-diagnostics --experiment rpi-firmware` | network only | waiting |
| 3 | Clock from NTP: `date` after boot, `timesync -n` ([experiment](../patches/experiments/timesync/README.md)) | timesync session | `--experiment ldso-diagnostics --experiment timesync` | network only | waiting |
| 4 | RTL8152 Ethernet over a USB adapter on the Pi 3B+ ([steps](../patches/experiments/rtl8152/README.md#a-pi-3b-with-a-usb-rtl8152-adapter)) | rtl8152 session | `--experiment ldso-diagnostics --experiment rtl8152` | person to plug in an RTL8152 USB adapter | waiting |

## Done

| Date | Test | Build ID | Result |
|---|---|---|---|
| 2026-10-27 | Standard checks, diagnostics image | `v0.1.0-alpha-14-gf39d20b+ldso-diagnostics` | [report](test-reports.md): all pass except `ps-tty` |
| 2026-10-27 | Standard checks, default image | `v0.1.0-alpha-14-gf39d20b` | [report](test-reports.md): 36 of 300 start failures |
