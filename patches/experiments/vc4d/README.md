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

QEMU (raspi3b has no HVS), image `v0.4.0-dev-74-g86cd4d9+x9736`
(netboot) with vc4d `1aacade`:

- Normal boot: the HVS reads as zeros, vc4d logs `the firmware left the HVS
  off (or this is QEMU, which has none)` and exits; `vesad` keeps the
  screen, Orbital runs on it.
- `VC4D_FAKE=640x480` and `VC4D_FAKE=1920x1200` (an HVS in memory): fbcond
  hands the console over to vc4d (`display.vc4`), then Orbital opens it and
  uses the cursor plane. The lists in the log: the primary plane at the
  full size with the buffer at a `0xc...` bus address, then the 64x64 ARGB
  cursor plane on top. At 1920x1200 all dumb buffers (9.2 MB) got
  contiguous memory.

Not shown by QEMU: whether the HVS takes the list, colours, the pointer
position, tearing. That is the Pi row (#220).

Images: netboot `keep/netboot-v0.4.0-dev-74-g86cd4d9+x9736.img` on
homenodellm, SD
http://homenodellm:8080/sdcard/redox-rpi-graphics-vc4d-v0.4.0-dev-74-g86cd4d9.img.xz
(sha256 `c9b241a2...31b8`).
