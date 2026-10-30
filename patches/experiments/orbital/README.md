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
- `v0.3.0-dev-195-g671b427+xdf3e` (terminal and keyboard round):
  - **Keyboard:** `dwc2d`'s keyboard events reach Orbital and the focused
    window without changes: `echo Hi` (with Shift) typed into `orbterm`
    ran and printed `Hi`. Super-F12 shows Orbital's FPS counter (12 FPS,
    94% CPU in QEMU). In one run, keys typed after a lone Super press were
    lost (QEMU's key timing during a busy desktop; check on the Pi). Left
    Super works as Orbital's Super key; `dwc2d` sends right Super as a
    different code, which Orbital ignores.
  - **Terminal:** the launcher's Super-T starts `cosmic-term`, which is
    not in the image; `config.toml` links `/usr/bin/cosmic-term` to
    `orbterm` (not yet seen working in QEMU: no run reached a ready
    desktop in time). `orbterm` started from the serial shell works.
  - **Terminal speed:** `ls -R /usr` (679 lines) takes 5.6 s in `orbterm`
    against 2.7 to 3.0 s to `/dev/null`, so about 4 ms per line for
    drawing. `orbterm` draws each character through `rusttype` with no
    glyph cache, scrolls in its own window buffer and sends the whole
    window on every update (1024x600, 2.4 MB): about 18 frames during the
    listing, 1.2 s of DMA blits in QEMU. Faster terminals need changes in
    upstream `orbterm`/`orbfont` (glyph cache, damage of the changed rows
    only), not in `rpi-fbd`.
  - **Memory:** 194 MiB used with the login window plus `orbterm`
    (`orbterm` 6 MB), against 140 MiB without Orbital.
  - **No mouse:** Super-Numpad-0 turns on Orbital's mouse keys (numpad
    8/4/6/2 move, 5 left click, 1 right click), a stopgap until `dwc2d`
    has a mouse.

- **Mouse** (branch `graphics-gui-mouse`: this branch merged with
  `usb-mouse`, #48; `--experiment timesync --experiment rpi-fb
  --experiment orbital --experiment usb-hotplug --experiment usb-mouse`,
  image `v0.3.0-dev-222-g67cef7a+xd99d`, QEMU with `-device usb-mouse`
  next to the keyboard, moves and clicks through QEMU's monitor):
  - `dwc2d` sends the mouse to `inputd` and Orbital takes it with no
    change in Orbital or `rpi-fbd`: the pointer moves, hovering over the
    launcher's terminal icon shows its `Terminal` tooltip, left and right
    buttons arrive (`dwc2d: mouse: buttons …` in the log).
  - With the test helper fixed (next point), the whole desktop works by
    keyboard and mouse (run `km1`, 2026-10-30): Enter at the login window,
    the desktop within 90 s, a click on the launcher's terminal icon opens
    `orbterm` as `user` within 90 s, and `echo hi` typed there prints
    `hi`. Memory (`free`, of 932 MiB): 194 MiB at the login window, 270
    MiB with the desktop and the terminal (`background` 82 MB with the
    decoded picture, `orbital` 25 MB, `launcher` 9 MB, `orbterm` 6 MB).
  - An earlier run's click opened nothing, and `getty` and `bcm2835-sdhci`
    used 11 to 12 minutes of CPU there; that was the same helper fault,
    not slow forks.
  - Some moves arrived late or merged with later ones, and logins took 2
    to 10 minutes in earlier runs. That was the test helper: while sending moves it read neither QEMU's serial output nor
    the monitor's replies, so `dwc2d` blocked writing its event log to the
    serial console. With both read, the moves arrive as sent; the USB
    session's own runs never showed it.

## What is missing

- **Mouse on the Pi:** in QEMU it works (see above); the Pi test with a
  mouse is in the USB topic's queue, and the NanoKVM's USB can act as one.
- **Pi numbers:** frame rate, the CPU against DMA trade with shadow
  buffers, the time from login to desktop, memory at 1920x1200. The Pi
  test in [docs/test-plan.md](../../../docs/test-plan.md) collects them.
- At 1920x1200 the 9 MiB dumb buffer needs that much contiguous memory;
  if there is none, `rpi-fbd` logs `in normal memory` and copies with the
  CPU (single buffered there anyway, see `graphics-fb2`).
