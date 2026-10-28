# rpi-firmwared, Raspberry Pi firmware properties

Named after the Linux driver `raspberrypi-firmware` (the old name was
`bcm2835-mboxd`, experiment `mailbox-driver`).

New driver that asks the Raspberry Pi firmware, which runs on the VideoCore
GPU, through the VideoCore mailbox (`brcm,bcm2835-mbox`, property
interface) and serves the answers as text files, read fresh on every open:

| File | Content |
|---|---|
| `/scheme/rpi-firmware/temperature` | SoC temperature in millidegrees Celsius |
| `/scheme/rpi-firmware/throttled` | Throttling flags as hex (bit 0 undervoltage, 1 frequency capped, 2 throttled, 3 soft temperature limit; bits 16–19 the same since boot) |
| `/scheme/rpi-firmware/clocks` | One line per clock: name, current rate, maximum rate (Hz) |
| `/scheme/rpi-firmware/board` | `key value` lines: revision, model, memory, serial, firmware, memory split, maximum temperature |
| `/scheme/rpi-firmware/arm_clock` | ARM clock in Hz (the requested rate); writing a rate in Hz, `max` or `min` sets it |

**ARM clock and throttle watch** (base patch
[`0021`](../patches/base/0021-rpi-firmwared-ARM-clock-to-its-maximum-arm_clock-fil.patch),
[#20](https://github.com/redox-rpi/redox-rpi/issues/20)): at start the driver
sets the ARM clock to the firmware's maximum (600 -> 1400 MHz on the 3B+),
which makes program starts twice as fast. The firmware raises the voltage
itself and lowers clock and voltage again at its temperature limits (soft
limit about 60 °C on the 3B+). A thread reads the throttling flags every 5
seconds and logs each change, for example
`rpi-firmwared: warning: soft temperature limit (flags 0x00080008, 60.1 °C, ARM 1400 MHz)`
and `rpi-firmwared: no longer throttled`. Tests and history:
[experiment `cpu-clock`](../patches/experiments/cpu-clock/README.md).

Part of every image since base patch
[`0018`](../patches/base/0018-rpi-firmwared-Raspberry-Pi-firmware-properties-over-.patch);
until then it was the experiment `rpi-firmware` (the results below are
from that time). The patch is numbered after the sdhost driver (experiment
`sdhost-driver`, which keeps `0017` for when it becomes a default patch),
and its changes to `Cargo.toml`, `Makefile` and `40_drivers.target` stay
clear of sdhost's lines, so both apply with and without each other.

Starts from the initfs, so other drivers (sdhost's clock divider) can ask
for the real core clock later.

**Result in QEMU** (`raspi3b`, build `v0.1.0-alpha-21-gb8b64d7+rpi-firmware`,
SD image, 2026-10-27): the driver starts from the initfs
(`rpi-firmwared: mailbox at 0x3f00b880, revision 0x00a02082, model
Raspberry Pi 3 Model B`) and all four files answer. QEMU reports fixed
values: revision `0x00a02082` (Pi 3 Model B, 1 GiB), 25.0 °C, ARM 700 MHz,
core 350 MHz, no throttling, serial 0. On the real Pi 3B+ see below.

**As a default patch in QEMU** (build `v0.1.0-alpha-59-gecabf31`, network
boot image without experiments, 2026-10-27): starts from the initfs, all
four files answer with QEMU's values as above.

**Result on the real Pi 3B+** (netboot, build
`v0.1.0-alpha-51-g3d7e351+ldso-diagnostics+rpi-firmware`, 2026-10-27): all
four files answer with real values: Raspberry Pi 3 Model B+ (revision
`0x00a020d3`), 1 GiB, 58.5 °C, ARM 600 MHz (max 1.4 GHz), **core 250 MHz**
(max 400 MHz), no throttling. The standard checks pass as with the
diagnostics image alone ([report](test-reports.md)). The core
clock confirms that sdhost's divider, which assumes 400 MHz, runs the card
slower than planned; with this driver it can ask for the real rate.
