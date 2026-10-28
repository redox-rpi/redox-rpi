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
- **`poweroff` tool:** next to `reboot` in [`tools/reboot`](../../../tools/reboot)
  (a second binary of the same package): flushes the file systems and writes
  `shutdown` to `kstop`. It is installed in every image, also without
  this experiment; there it only stops the CPU, as before.

QEMU's `raspi3b` does not model partition 63: there the watchdog reset is
a reboot. So QEMU only shows that the kernel takes the new path.

## Test on the Pi

Needs a person to cut and reconnect the power at the end.

1. Boot, `scripts/pi-test.py` standard checks (also checks `reboot`).
2. `poweroff` over devsh. The serial log ends with `kstop`, nothing after
   it; the green LED stays off, the Ethernet LEDs go out, and ping gets no
   answer for 2 minutes.
3. Cut the power and reconnect it: the Pi boots normally.

## Results

- Not tested yet.
