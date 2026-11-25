# Experiment v3d-power

V3D on the Pi 4 and 400 (#598). The firmware leaves the BCM2711 V3D power
domain off at boot (Pi test 547: `PM_GRAFX` 0x1000, no power). Linux
powers it through the firmware (raspberrypi-power, `RPI_POWER_DOMAIN_V3D`).
This experiment adds one base patch:

- `rpi-firmwared`: `/scheme/rpi-firmware/v3d_power` reads `on` or
  `off`; writing `on` or `off` switches the domain (mailbox tags
  `GET_DOMAIN_STATE` 0x30030, `SET_DOMAIN_STATE` 0x38030, domain 10). Only
  on BCM2711 boards; elsewhere the file answers ENOENT.

vc4d's `v3dd` and `v3dprobe` (experiment v3dd) write `on` when GRAFX has
no power; `v3dprobe --off` writes `off` again.

It must come after experiment pi400, whose patches change the same file:

```
scripts/build-image.sh --experiment pi400 --experiment v3d-power --experiment v3dd ...
```

Pi tests: 547 (power off at boot), 584 onwards with experiment v3dd.
