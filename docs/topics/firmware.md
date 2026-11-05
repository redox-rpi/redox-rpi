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
- `cputop` ([#58](https://github.com/redox-rpi/redox-rpi/issues/58)): CPU
  time per core and per thread over an interval, to find what keeps a core
  busy. It found the idle Pi for the ondemand governor (#23, Pi tests #82
  and #93). PR [#210](https://github.com/redox-rpi/redox-rpi/pull/210)
  (branch `cputop-58`) adds kernel patch `cputop` 0001 (`sys:cputime`:
  exact CPU time and wakeups per thread), a `WAKE/s` column, `-w`, and
  interrupts per second. Pi test #239, 5 min idle on stable: cpu1 2 %
  busy, the rest about 0 %. `dwc2d` 2.5 % and 76 wakeups/s; `netstack`
  27/s; `rpi-firmwared` 0.3 %, 9/s. Interrupts 108/s: timer 100, line 50
  7–8.
- Power-saving mode ([#126](https://github.com/redox-rpi/redox-rpi/issues/126),
  v0.5.0), branch `power-save`, experiment `power-save`: `rpi-firmwared`
  `display` (HDMI power off/on, blank) and `power_domains`. Pi test 147:
  idle 3.23 W on the stable image, 2.99 W on current main (the ondemand
  governor, #23). Blanking, HDMI off, both LEDs off, the unused power
  domains and `powersave` each changed less than 0.1 W, the plug's
  resolution; the domains were already off at boot. Idle cores already
  sleep (`wfi` in the kernel's idle loop). What still wakes them most is
  `dwc2d` (2.4 % of a core, about 290 switches/s), which #104 works on.
  Plan for the rest on #126 (2026-11-04): step 1 Ethernet at 100 Mbit/s
  instead of gigabit (branch `power-eth`, experiment `eth-speed`:
  `echo 100 > /tmp/dwc2d-link` makes `dwc2d` renegotiate; QEMU 4 of 4,
  Pi test #228: **100 Mbit/s saves about 0.35 W at idle**, 3.23 W against
  2.89 W). Step 2 (same branch): `/etc/eth-speed` sets the speed at boot,
  and `rpi-firmwared` `clock_states` (experiment `clock-states`) shows
  which firmware clocks run (Pi tests #231, #232, #234; PR
  [#203](https://github.com/redox-rpi/redox-rpi/pull/203)). `dwc2d` idle
  wakeups: [#211](https://github.com/redox-rpi/redox-rpi/issues/211), USB
  session, experiment `dwc2d-idle`, draft PR
  [#212](https://github.com/redox-rpi/redox-rpi/pull/212). What wakes it:
  the 1 s hotplug rescan, the 10 ms keyboard poll, netstack's reads on each
  tick, and LAN7800 interrupts (SOF is already masked). The change rescans
  every 3 s, reads the link every 5 s, and counts wakeups by source in
  `/tmp/dwc2d-wakeups`. Pi test #242: no change (2.5 % and about 76
  wakeups/s before and after). The cost is the 10 ms keyboard poll: 50/s,
  about 130 µs each, 2.2 % of a core. The test Pi's low-speed keyboard
  sits behind a hub, so each poll is a split transaction that `dwc2d`
  spins on. Pi test #243 tried a keyboard poll that waits on the
  controller interrupt, and it failed. `dwc2d` CPU went down to 1.2 %, but
  its wakeups went up to 240–480/s, interrupts on line 50 to 170–410/s and
  idle-thread wakeups to 460/s. 619 of 25984 polls never finished, and
  `dwc2d` hung in the hotplug test. The reason: a split transaction needs
  answers from user space within microframes, about 7 interrupts per poll.
  The change was dropped; PR #212 keeps only the slower rescan and the
  counters. So with the low-speed keyboard plugged in, idle stays at about
  2.2 % of a core; without a keyboard it is much less. Idle wakeups rose
  after row 239 (`dwc2d` 76/s to about 113/s, `netstack` 28/s to about
  63/s). Pi test #246 compared three images: before #203 and #194, after
  #203 with `eth-speed`, and after #194. `eth-speed` adds about 3 `dwc2d`
  wakeups/s. #194 adds about 28 each for `dwc2d` and `netstack`, and 10
  USB interrupts/s, while CPU barely changes. The cause is base `0084`:
  the LAN7800 now accepts *all* multicast (`RFE_CTL_MCAST_EN`), because
  netstack doesn't tell the driver its groups. So every mDNS, SSDP and
  IPv6 frame on the LAN wakes both. That's traffic, not a busy loop. A
  hash filter with only the joined groups cuts it ([#216](https://github.com/redox-rpi/redox-rpi/issues/216)):
  Network's PR [#218](https://github.com/redox-rpi/redox-rpi/pull/218)
  (base `0085`: netstack tells the driver its groups) and the LAN7800
  hash filter in PR [#219](https://github.com/redox-rpi/redox-rpi/pull/219)
  (experiment `lan78xx-mcast-hash`, stacked on #218). Pi test #247, A/B:
  `dwc2d` 108 to 81 wakeups/s, `netstack` 60 to 30, USB interrupts 19 to
  8, back to the level from before #194. Before it can be the default,
  the `ipv6` experiment has to add its groups (Network). PR
  [#215](https://github.com/redox-rpi/redox-rpi/pull/215) saves about 6
  `dwc2d` wakeups/s (Pi test #245). Then EEE on the LAN7800 and gating clocks that run for nothing.

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
