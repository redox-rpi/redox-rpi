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
| HDMI off | `display` (this patch) | to measure |
| Bluetooth and Wi-Fi off | `bt_power`, `wifi_power` (experiments `bluetooth`, `wifi`) | off at boot unless a driver switches them on (Pi reports: `wifi_power was 0`) |
| LEDs off | `power_led` (base `0046`), activity LED GPIO 29 through `/scheme/gpio/29/value` | to measure |
| Unused power domains off | `power_domains` (this patch) | to measure |
| ARM clock follows the load | governor, base `0040` (#23) | default; Pi test 93: 3.05 W idle, 3.00 W `powersave`, 3.20 W `performance` |

## QEMU

`qemu-check.sh`: reads both files, switches `i2c2` off and on, the display
off, on, blank and unblank, and checks that `usb` and `uart0` are refused.
QEMU has no real domains or HDMI, so it checks the driver, not the saving.
