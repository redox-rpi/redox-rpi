# Firmware interface

## State

- `rpi-firmwared` (base patch `0018`) talks to the VideoCore firmware over
  the mailbox and serves `/scheme/rpi-firmware`: model, memory, clocks,
  temperature, throttling. See [rpi-firmware.md](../rpi-firmware.md).
- The ARM clock runs at its maximum (1400 MHz on the 3B+, twice as fast as
  600 MHz), and undervoltage or overheating is logged (base patch `0021`,
  [#20](https://github.com/redox-rpi/redox-rpi/issues/20), Pi tests #13 and
  #19: 0 of 7000 starts failed at 1400 MHz with the x18 fix).
- `reboot` through the watchdog (kernel patch `0002`).

## In progress

<<<<<<< HEAD
- [#23](https://github.com/redox-rpi/redox-rpi/issues/23) the CPU clock follows the load (v0.4.0): experiment
  [`cpu-ondemand`](../../patches/experiments/cpu-ondemand/README.md), base
  patch `0023`, branch `cpu-ondemand`. Waits for its Pi test.
=======
- Bluetooth, [#54](https://github.com/redox-rpi/redox-rpi/issues/54)
  (outlook for v0.9.0), see below.
- `cputop` ([#58](https://github.com/redox-rpi/redox-rpi/issues/58), branch
  `cputop`): CPU time per core and per thread over an interval, to find
  what keeps a core busy. It is for the ondemand governor (#23), which saw
  no idle Pi in test #25; Pi test #82 uses it.

## Bluetooth

The Bluetooth half of the Pi 3B+ Wi-Fi/Bluetooth chip (CYW43455,
Bluetooth part BCM4345C0) sits on a UART at GPIO 30–33. The serial console
keeps the PL011 (`dtoverlay=disable-bt`), so Bluetooth gets the mini UART
(ALT5, RTS/CTS), as with Linux's `miniuart-bt` overlay. BT_ON is pin 0 of
the firmware's GPIO expander.

Branch `bluetooth`, experiment `bluetooth` (needs `gpio`):

- `rpi-firmwared` `bt_power` (BT_ON).
- Driver `bcm2835-btuartd`: `/scheme/btuart` (`data`, `baud`, `clock`).
- Tool `bt`:
  - `up`: power cycle, firmware `BCM4345C0.hcd` upload, reset.
  - `info`
  - `scan` (LE)
  - `inquiry` (classic)
  - `cmd`
- Firmware downloaded at build time (package `bt-firmware`).

QEMU is tested against a fake HCI controller on the mini UART. Next is a
Pi test on the serial console only.

Not yet: a Bluetooth stack (L2CAP, pairing, GATT), Zero 2 W (BCM43436),
higher baud rates. The mini UART's baud rate follows the core clock, so
throttling garbles it.
>>>>>>> origin/main

## Later

- Throttle watch: fewer lines for the soft temperature limit (59 in 30
  minutes under load); show the measured ARM clock (tag `0x30047`) next to
  the requested one.
- [#21](https://github.com/redox-rpi/redox-rpi/issues/21) a common DMA plan for the Pi, including the BCM2835 DMA
  controller and the firmware's channel mask (milestone v0.4.0).
- [#7](https://github.com/redox-rpi/redox-rpi/issues/7) power off, v0.5.0 or later (not needed for now).
- Power domains, GPIO expander, DMA channel mask, display, VCHIQ
  ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
