# Experiment vc4d

Stage 1 of [#188](https://github.com/redox-rpi/redox-rpi/issues/188): the
VideoCore IV display driver `vc4d` with HVS planes, instead of `rpi-fbd`.

`vc4d` is GPL-2.0-only and lives in its own repository,
[redox-rpi/vc4d](https://github.com/redox-rpi/vc4d). This experiment holds
no vc4d source: only a recipe that builds a pinned commit
(`recipes/other/vc4d/recipe.toml`), the start file and the kernel patches
for large contiguous buffers (`kernel/`, links to experiment `rpi-fb`'s).

```
scripts/build-image.sh --experiment vc4d --experiment orbital
```

Not together with `--experiment rpi-fb`: `rpi-fbd` would take the display
first. Without it, `vesad` shows the boot framebuffer until `vc4d` starts
(`/usr/lib/init.d/15_vc4d.service`, before Orbital).

## What vc4d does

The firmware still sets up HDMI and the mode at power-on and leaves one HVS
channel running. `vc4d` takes that channel over with its own display list:
the client's dumb buffer is scanned out directly as the primary plane (no
copy into a firmware framebuffer), and a 64x64 cursor plane is Orbital's
pointer. Details in the vc4d README.

## Results

See the sections below as they come; the Pi check is a test-plan row (HDMI,
NanoKVM or a person).
