# Experiment: poweroff

Issue [#7](https://github.com/redox-rpi/redox-rpi/issues/7), planned for
v0.5.0.

```bash
scripts/build-image.sh --experiment poweroff
```

The Raspberry Pi 3 has no power switch and its firmware no PSCI, so
`SYSTEM_OFF` fails and the kernel only stopped the CPU: the firmware, the
USB hub and the Ethernet chip kept running, and only a reset or cutting
the power brought the board back.

- **Kernel patch `0007`:** when PSCI is missing, `kstop` (writing
  `shutdown` to `/scheme/sys/kstop`) marks partition 63 in the watchdog's
  `PM_RSTS` register and resets through the watchdog, like Linux's
  `bcm2835_power_off`. The firmware sees the mark and halts instead of
  starting again: the green LED goes out, USB and Ethernet lose power
  (the red power LED stays on). The board starts again only when the power
  is cut and reconnected.
- **`poweroff` and `reboot`** ([`tools/reboot`](../../../tools/reboot), two
  binaries of one package, installed in every image) now shut down in
  order before they call `kstop`:
  1. SIGTERM to the programs started after boot (everything not from the
     initfs except `ptyd` and RedoxFS: shells, `login`, `getty`, `devshd`,
     `timesync`, …), SIGKILL after 5 seconds to those still there;
  2. one second for the SD card to finish writing;
  3. `kstop`.

  The terminal gets one line (`stopping the system, the rest is on the
  serial console`); the steps are reported on the serial console only,
  because the terminal is a pty that `getty` holds, and once `getty` is
  gone a write to it never returns. No file system step: `sync()` does
  nothing in relibc, and RedoxFS needs none, as each change is a
  transaction that writes its blocks and then a new header before the call
  returns (an unmount through RedoxFS's SIGTERM handler was tried and
  hung). Without the kernel patch, `poweroff` only stops the CPU, as
  before.

QEMU's `raspi3b` models partition 63: with the mark, the watchdog reset
ends QEMU instead of restarting the guest.

## Test on the Pi

Needs a person to cut and reconnect the power at the end.

1. Boot, `scripts/pi-test.py` standard checks (also checks `reboot`).
2. `poweroff` over devsh. The serial log shows `poweroff: N programs
   stopped, M killed` and `poweroff: shutdown through kstop`, then `kstop`,
   nothing after it; the green LED stays off, the Ethernet LEDs go out, and ping gets no
   answer for 2 minutes.
3. Cut the power and reconnect it: the Pi boots normally.

## Results

- **QEMU** (build `v0.3.0-dev-95-gf6e2070+x7098`, 2026-10-28):
  `/usr/bin/poweroff` is installed next to `reboot`; `poweroff` prints
  `kstop` and QEMU exits (a reset would have started U-Boot again). Ready
  for the Pi test.
- **QEMU, clean shutdown** (2026-10-29, build `v0.3.0-dev-105-g9359678+x7098`, [`qemu-check.sh`](qemu-check.sh)):
  after the boot, `poweroff` reports `7 programs stopped, 1 killed` and
  `shutdown through kstop`, and QEMU exits without a restart. On the way
  there: a write to the terminal after `getty` had ended blocked forever
  (now reports go to the serial console), and the RedoxFS unmount hung
  (dropped). `getty` and `fbcond` panic on the interrupted read when they
  get SIGTERM; harmless here, as they are meant to end.
