# Experiment: power-save

Issue [#126](https://github.com/redox-rpi/redox-rpi/issues/126), planned for
v0.5.0: lower the Pi's idle power, as options for a headless server. This
experiment adds switches to measure each idea with the Shelly plug; the ones
that help later become options in a config file.

```
scripts/build-image.sh --experiment power-save [--experiment cputop]
```

## Base patch `0070`: `rpi-firmwared` `display` and `power_domains`

- `/scheme/rpi-firmware/display`: `echo off > …` switches the HDMI
  display's power off (firmware tag `SET_DISPLAY_POWER` 0x48019, as Linux's
  vc4 driver does), `on` back on; `blank`/`unblank` only blanks the picture
  (`FRAMEBUFFER_BLANK` 0x40002). Reading gives the last setting (`on` at
  start).
- `/scheme/rpi-firmware/power_domains`: one line per firmware power domain
  (`GET_POWER_STATE` 0x20001, device ids 0–8: `sd uart0 uart1 usb i2c0
  i2c1 i2c2 spi ccp2tx`), `on` or `off`; domains the firmware does not have
  are left out. `echo "<name> off" > …` switches one (`SET_POWER_STATE`
  0x28001, waiting until done). `usb` and `uart0` are refused with
  `EACCES`: on a 3B+ the network hangs on USB and the serial console on the
  PL011.

## The ideas of #126 and where they stand

| Idea | How it is switched | State |
|---|---|---|
| Idle cores sleep | kernel idle loop: `wfi` when every context is idle (`enable_and_halt` in `startup/mod.rs`) | already so; `cputop` shows who keeps a core awake |
| HDMI off | `display` (this patch) | Pi test 147: blank −0.08 W, off +0.08 W: no change (plug resolution 0.1 W) |
| Bluetooth and Wi-Fi off | `bt_power`, `wifi_power` (experiments `bluetooth`, `wifi`) | off at boot unless a driver switches them on (Pi reports: `wifi_power was 0`) |
| LEDs off | `power_led` (base `0046`), activity LED GPIO 29 through `/scheme/gpio/29/value` | Pi test 147: 0 and +0.03 W: no change |
| Unused power domains off | `power_domains` (this patch) | Pi test 147: already off at boot (`uart0 uart1 i2c0 i2c1 i2c2 spi ccp2tx`); the firmware answers state 0 to `on` too, so they cannot be switched on this way either |
| ARM clock follows the load | governor, base `0040` (#23) | default; Pi test 93: 3.05 W idle, 3.00 W `powersave`, 3.20 W `performance`; Pi test 147: `powersave` +0.01 W |

## Result on the Pi (test 147)

Shelly plug, 18 readings over 3 minutes per sample, nothing else running:
idle **3.23 W** on the stable image (`v0.3.0-dev-533`), **2.99 W** on this
image (current main with the ondemand governor). No switch changed the
power by 0.1 W or more. `cputop 30`: cores 0–2 % busy, 107–291
switches/s; `dwc2d` takes 2.4 % of a core, the largest part. So nothing
here becomes an option; the experiment stays on its branch as a tool for
later measurements.

## QEMU

`qemu-check.sh`: reads both files, switches `i2c2` off and on, the display
off, on, blank and unblank, and checks that `usb` and `uart0` are refused.
QEMU has no real domains or HDMI, so it checks the driver, not the saving;
its firmware lists no power domains at all. `v0.3.0-dev-703-g7ce5389+x9024`
(`--experiment power-save --experiment cputop`): 10 of 10, cputop 6 of 6.
