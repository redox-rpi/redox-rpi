# Firmware interface

## State

- `rpi-firmwared` (base patch `0018`) talks to the VideoCore firmware over
  the mailbox and serves `/scheme/rpi-firmware`: model, memory, clocks,
  temperature, throttling. See [rpi-firmware.md](../rpi-firmware.md).
- The ARM clock runs at its maximum (1400 MHz on the 3B+, twice as fast as
  600 MHz), and undervoltage or overheating is logged (base patch `0021`,
  [#20](https://github.com/redox-rpi/redox-rpi/issues/20), Pi tests #13 and
  #19: 0 of 7000 starts failed at 1400 MHz with the x18 fix).
- The ARM clock follows the load (ondemand governor in `rpi-firmwared`,
  base patch `0040`, kernel patch `0040` for exact CPU times;
  [#23](https://github.com/redox-rpi/redox-rpi/issues/23), Pi test #93):
  600 MHz at idle 96 % of the time, 1400 MHz under load, program starts as
  fast as with the fixed maximum. `/scheme/rpi-firmware/governor` shows
  the mode and the time at each rate; `performance` and `powersave` fix
  the clock. See [the experiment notes](../../patches/experiments/cpu-ondemand/README.md).
- The mode at boot comes from `/etc/rpi-firmware/governor` if it exists
  (`ondemand`, `performance`, `powersave`, or a rate in Hz, `max`, `min`
  for a fixed clock), set by `15_rpi-governor.service` with
  `/usr/lib/rpi-firmware/governor.ion` (`config/raspi3bp-sdcard.toml`,
  Pi test #161). Without the file the clock follows the load.
- `reboot` through the watchdog (kernel patch `0002`).

## In progress

- Bluetooth, [#54](https://github.com/redox-rpi/redox-rpi/issues/54)
  (outlook for v0.9.0), see below.
- `cputop` ([#58](https://github.com/redox-rpi/redox-rpi/issues/58), branch
  `cputop`): CPU time per core and per thread over an interval, to find
  what keeps a core busy. It found the idle Pi for the ondemand governor
  (#23, Pi tests #82 and #93).
- Power-saving mode ([#126](https://github.com/redox-rpi/redox-rpi/issues/126),
  v0.5.0), branch `power-save`, experiment `power-save`: `rpi-firmwared`
  `display` (HDMI power off/on, blank) and `power_domains`. Pi test 147:
  idle 3.23 W on the stable image, 2.99 W on current main (the ondemand
  governor, #23). Blanking, HDMI off, both LEDs off, the unused power
  domains and `powersave` each changed less than 0.1 W, the plug's
  resolution; the domains were already off at boot. Idle cores already
  sleep (`wfi` in the kernel's idle loop). What still wakes them most is
  `dwc2d` (2.4 % of a core, about 290 switches/s), which #104 works on.

## Bluetooth

The Bluetooth half of the Pi 3B+ Wi-Fi/Bluetooth chip (CYW43455,
Bluetooth part BCM4345C0) sits on a UART at GPIO 30–33. The serial console
keeps the PL011 (`dtoverlay=disable-bt`), so Bluetooth gets the mini UART
(ALT5, RTS/CTS), as with Linux's `miniuart-bt` overlay. BT_ON is pin 0 of
the firmware's GPIO expander.

Branch `bluetooth`, experiment `bluetooth` (GPIO is default since PR #87):

- `rpi-firmwared` `bt_power` (BT_ON), `wifi_power` (WL_ON), `core_clock`.
- Driver `bcm2835-btuartd`: `/scheme/btuart` (`data`, `baud`, `clock`).
- Tool `bt`:
  - `up`: power cycle, firmware `BCM4345C0.hcd` upload, reset.
  - `info`
  - `scan` (LE)
  - `inquiry` (classic)
  - `cmd`
- Firmware downloaded at build time (package `bt-firmware`).

QEMU is tested against a fake HCI controller on the mini UART. On the Pi
(row 109), the chip needs WL_ON on, and since step 8 it answers HCI Reset
when the baud divider assumes a 400 MHz core clock, while the firmware
reports 250 MHz. So `rpi-firmwared` gets `core_clock` (the measured rate),
and a clock written to `/scheme/btuart/clock` by hand stays through
`bt up`. Step 9 confirmed the measured 400 MHz and also showed the chip
wants the flow control lines asserted low (`auto-low`, now the start
setting) and that the receive thread lost bytes between sleeps. Step 10:
`bt info` answers (BCM4345C0, build 0) with no bytes lost; `bt up` sent
its Reset too soon after power-on. Step 11 waited longer and retried, but
the chip stayed silent, and so did `bt info` after it: `bt up` never
switched WL_ON on, only `bt diag` did. Step 12 switches WL_ON on first and
**passes**: firmware loaded (323 records in 5.9 s, build 382, Bluetooth
5.0), address `43:45:C0:00:1F:AC`, LE scan 22 devices, no bytes lost.
Draft PR [#136](https://github.com/redox-rpi/redox-rpi/pull/136) brings the
experiment to main (base `0071`–`0081`).

Not yet: a Bluetooth stack (L2CAP, pairing, GATT), Zero 2 W (BCM43436),
higher baud rates. The mini UART's baud rate follows the core clock, so
throttling garbles it.

## Later

- Throttle watch: fewer lines for the soft temperature limit (59 in 30
  minutes under load); show the measured ARM clock (tag `0x30047`) next to
  the requested one.
- [#21](https://github.com/redox-rpi/redox-rpi/issues/21) a common DMA plan for the Pi, including the BCM2835 DMA
  controller and the firmware's channel mask (milestone v0.4.0).
- [#7](https://github.com/redox-rpi/redox-rpi/issues/7) power off, v0.5.0 or later (not needed for now).
- Power domains, GPIO expander, DMA channel mask, display, VCHIQ
  ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
