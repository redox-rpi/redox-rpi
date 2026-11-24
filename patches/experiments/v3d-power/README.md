# Experiment v3d-power

V3D step 0 on the Pi 4 and 400 (vc4d `v3dprobe`). The firmware leaves the
BCM2711 V3D power domain off at boot (Pi test 547: `PM_GRAFX` 0x1000, no
power). Linux powers it through the firmware (raspberrypi-power,
`RPI_POWER_DOMAIN_V3D`). This experiment adds one base patch:

- `rpi-firmwared`: `/scheme/rpi-firmware/v3d_power` reads `on` or
  `off`; writing `on` or `off` switches the domain (mailbox tags
  `GET_DOMAIN_STATE` 0x30030, `SET_DOMAIN_STATE` 0x38030, domain 10). Only
  on BCM2711 boards; elsewhere the file answers ENOENT.

`v3dprobe` (vc4d `v3dprobe` branch, experiment vc4d-flip) writes `on`
when GRAFX has no power, and `off` again with `--off`.

It must come after experiment pi400, whose patches change the same file:

```
scripts/build-image.sh --experiment pi400 --experiment v3d-power --experiment vc4d --experiment vc4d-flip ...
```
