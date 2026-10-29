# Experiment: orbital, Redox's desktop on the Pi (#33)

An effort probe from the [graphics topic](../../../docs/topics/graphics.md):
Orbital, Redox's display server and window manager, with its login window,
launcher and terminal, on top of `rpi-fbd` (experiment `rpi-fb`).

```bash
scripts/build-image.sh --experiment timesync --experiment rtl8152 \
    --experiment dwc2-interrupts --experiment rpi-fb --experiment orbital
```

`--experiment rpi-fb` is needed for the display; `dwc2-interrupts` (a
default patch on main since `dwc2-default`) keeps `dwc2d` from spending a
whole core on polling the keyboard.

- [`config.toml`](config.toml): the packages and start file of upstream's
  `config/desktop-minimal.toml`: `orbital`, `orbterm`, `orbutils`,
  `orbdata` (3 MB of fonts, icons and backgrounds), taken as binary
  packages from `static.redox-os.org` like the rest of the image. Orbital
  runs on VT 3 with `orblogin` and then the launcher; the switch to the
  text console on VT 2 is turned off. The serial console keeps its login.
- Two `rpi-fbd` patches it needs, in experiment `rpi-fb` (branch
  `graphics-gui`):
  - [`0028`](../rpi-fb/base/0028-rpi-fbd-remember-only-the-frame-s-own-damage-for-the.patch):
    with double buffering, `rpi-fbd` stored the union of this and the
    last frame's damage for the other buffer, so it grew until every
    frame copied the whole screen. Found with Orbital, whose frames were
    all 3.7 MB at 1280x720. Present since `graphics-fb2` (#43's image):
    costs speed, not correctness.
  - [`0029`](../rpi-fb/base/0029-rpi-fbd-shadow-buffers-when-dumb-buffers-are-uncache.patch):
    with DMA blits the dumb buffers are uncached, and Orbital composes
    (blends windows, reads back) in the buffer it gets. `rpi-fbd` now asks
    clients for a shadow buffer in normal memory then (`prefer_shadow`);
    `graphics-ipc` copies the changed part over, which only writes to the
    uncached memory. Also builds `drawbench` (in base, not built by
    default): 100 frames of a 512x512 rectangle through `MODE_DIRTYFB`,
    the time printed, a frame rate for the whole path from client to
    screen.

## Results

QEMU `raspi3b` (4 cores, 1 GiB), 2026-10-29, `RPI_FB=1280x720` typed
into the bootloader's editor, helper `build/fbgui.py` on homenodellm
(screenshots, serial commands, keys through QEMU's USB keyboard):

- `v0.3.0-dev-191-g2d7a1b2+xc786` (packages only): Orbital's login window
  at 1280x720 on `rpi-fbd`, DMA blits. `dwc2d` and `bcm2835-sdhci` each
  spent about 45 s of CPU time polling by the login (without
  `dwc2-interrupts`).
- `v0.3.0-dev-191-g2d7a1b2+xdf3e` (with `dwc2-interrupts`): `dwc2d` 1.4 s.
  Enter in the login window logs `user` in; the desktop (launcher bar,
  then the background picture) appears 1 to 3 minutes later in QEMU, so
  keys sent in that time went nowhere and no terminal was opened by
  keyboard. Every frame was the whole screen (3.7 MB): the bug fixed by
  `0028`.
- `v0.3.0-dev-193-g2983d13+xdf3e` (with `0028` and `0029`): frames are
  now partial (6 frames, 8.4 MB in one 5 s report). Desktop after login
  within 200 s. Memory (`free`, of 932 MiB): 140 MiB used without Orbital
  (`graphics-fb3` image), 165 MiB with the login window, 159 MiB with the
  desktop, so Orbital adds about 20 to 25 MiB; `ps` shows `orbital` 20 MB,
  `launcher` 9 MB, `background` 84 MB private (the picture, decoded),
  `orblogin` 9 MB. `drawbench` on VT 4 (`drawbench > /tmp/db.txt &`,
  `inputd -A 4`): 160 to 280 frames of 512x512 in 5 s, all blitted by DMA,
  so 30 to 57 frames per second, set by QEMU's slow DMA model; not a Pi
  number. Back on Orbital with `inputd -A 3`.
- QEMU shows red and blue swapped (its pixel order quirk, as before).

## What is missing

- **Mouse:** `dwc2d` drives one USB keyboard in the boot protocol and no
  mouse, so Orbital can only be used by keyboard (login, the shortcuts
  shown while Super is held). A boot-protocol mouse is the same kind of
  code as the keyboard (`dwc2d/src/hid.rs`, 300 lines) plus relative
  motion events to `inputd`; one device of each kind until hotplug and
  more channels (`usb-hotplug` branch).
- **Pi numbers:** frame rate, the CPU against DMA trade with shadow
  buffers, the time from login to desktop, memory at 1920x1200. The Pi
  test in [docs/test-plan.md](../../../docs/test-plan.md) collects them.
- At 1920x1200 the 9 MiB dumb buffer needs that much contiguous memory;
  if there is none, `rpi-fbd` logs `in normal memory` and copies with the
  CPU (single buffered there anyway, see `graphics-fb2`).
