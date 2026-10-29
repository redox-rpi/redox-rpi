# Experiment: rpi-fb, display through the Pi firmware (#33)

Step 1 of the [graphics topic](../../../docs/topics/graphics.md): a display
driver that gets its framebuffer from the Pi firmware itself, instead of
only inheriting the boot framebuffer.

```bash
scripts/build-image.sh --experiment timesync --experiment rtl8152 --experiment rpi-fb
```

- [`0024`](base/0024-rpi-firmwared-raw-property-file-for-other-drivers.patch):
  `rpi-firmwared` gets a `property` file (root only, not listed): a driver
  writes a whole property message, reads back the firmware's answer. So
  only `rpi-firmwared` uses the mailbox.
- [`0025`](base/0025-rpi-fbd-Raspberry-Pi-display-through-the-firmware.patch):
  the new driver `rpi-fbd` (initfs, after `rpi-firmwared`).
- [`0026`](base/0026-rpi-fbd-EDID-polling-double-buffering-screen-blankin.patch)
  (branch `graphics-fb2`): EDID polling, double buffering, screen blanking.

## What `rpi-fbd` does

- At start it reads the monitor's EDID (tag `0x00030020`, base block and
  up to 3 extensions, checked for header and checksum) and the display
  size the firmware chose at power-on.
- It keeps the boot framebuffer (`FRAMEBUFFER_*` from the Redox bootloader
  or `rpi-boot-stub`) when the firmware still shows it at that size, so the
  handover from `vesad` does not flicker. Without one it asks the firmware
  for a new framebuffer of the display size (fallback 1024x768).
- It registers `display.rpi` as a regular display, so `fbcond` and
  `fbbootlogd` hand the console over from `vesad` (an early display), the
  same way `virtio-gpud` takes over on x86.
- Modes: first the display size (or `RPI_FB=WIDTHxHEIGHT`), then the EDID's
  detailed timings, then common sizes up to 1920x1200 (16 MiB of GPU
  memory with `gpu_mem=16`). A mode set by a client allocates a new
  framebuffer of that size; **the firmware scales it to the monitor's mode,
  which stays the one it chose at power-on**.
- `RPI_FB=off` leaves the display to `vesad`. The stub reads the same
  variable from `cmdline.txt`; on the U-Boot path it goes in with `e` at
  the bootloader menu.

### Without a monitor

The connector is always reported as connected. Without a monitor the
firmware still has a mode (its fallback, 640x480 seen once on the test
Pi; composite video on the 3B+), `rpi-fbd` logs `no EDID` and uses that
size, and the boot goes on as before. A monitor plugged in later gets that
mode. Real hotplug would need EDID polling (not done).

### Branch `graphics-fb2` (patch `0026`)

- **EDID polling:** every 5 seconds (timer from `/scheme/time`) the EDID is
  read again. When it changes, the log says `rpi-fbd: monitor connected:
  NAME` or `monitor gone`, the connector gets the new EDID and modes, and
  clients get a hotplug event (`fbcond` then sets up its display again).
  The HDMI mode stays the one from power-on: with `hdmi_force_hotplug=1`
  a monitor turned on later shows the firmware's fallback mode, and the
  poll only makes Redox notice it. Whether the firmware reads the EDID
  again after power-on, or answers from what it read then, is unknown
  until the Pi test.
- **Double buffering:** the framebuffer gets two screens one below the
  other (virtual height twice the height). `set_plane` copies the changed
  rectangle, plus what the last frame changed in the other screen, into
  the hidden screen and moves the visible window there (tag
  `0x00048009`). Used only when both screens take at most half of the GPU
  memory: with `gpu_mem=16` that is up to about 1280x800; at 1920x1200
  (9 MiB per screen) it stays single buffered and keeps the boot
  framebuffer. A failed request for two screens falls back to one.
  The firmware applies the offset at its next frame; `rpi-fbd` does not
  wait for that, so a frame drawn right after a flip can still tear.
- **Screen blanking:** a client that sets no mode on the display (the KMS
  way to turn it off) blanks the screen through the firmware (tag
  `0x00040002`), the next mode turns it on. `fbcond` never does that, so
  nothing triggers it yet; the DPMS property of driver-graphics is
  read-only.

### Not done

- Waiting for the vertical blank before the next frame after a flip.
- A new HDMI mode for a monitor turned on later (needs the display
  hardware itself, step 2).
- `vesad` keeps writing to the old buffer until the handover; after a
  mode change the firmware may have moved it.

## Results

- QEMU `raspi3b`, `v0.3.0-dev-132-gae30bad+xb2e2` (`--experiment timesync
  --experiment rtl8152 --experiment rpi-fb`), 2026-10-29:
  - U-Boot path: `rpi-fbd: no EDID` (QEMU has no EDID tag), `rpi-fbd: boot
    framebuffer 640x480 stride 640 at 0x3c100000`, `fbbootlogd: handoff
    requested`, `fbcond: Performing handoff`, login prompt.
  - U-Boot path with `RPI_FB=1280x720` typed into the bootloader's
    environment editor: after the handover `rpi-fbd: 1280x720 stride 1280
    at 0x3c100000`; the screen is 1280x720 and shows the console.
  - Direct boot (`scripts/fw-direct-boot.sh` with this image, the stub
    from branch `graphics-fb`): the stub's 640x480 framebuffer is kept,
    handover, login prompt.
  - QEMU shows red and blue swapped on every path, U-Boot's too: its
    model reads the pixel order the other way round from the firmware.
- QEMU, branch `graphics-fb2`, `v0.3.0-dev-178-gaddaafc+xb2e2`
  (2026-10-29): `rpi-fbd: 640x480 stride 640 at 0x3c100000, double
  buffered` (QEMU's GPU memory is 64 MiB), handover, login; the boot log
  scrolls through hundreds of small updates and the screen shows it
  without gaps. With `RPI_FB=1280x720`: `rpi-fbd: 1280x720 stride 1280 at
  0x3c100000, double buffered`, screen correct. QEMU has no EDID tag, so
  only the "no change" side of the polling runs; blanking has no trigger.
  Keys on QEMU's USB keyboard did not show on any console (not checked
  further).
- The image above was built at commit `ae30bad`, when the patches were
  still numbered `0023` and `0024` (same content).
- Real Pi: queued in [docs/test-plan.md](../../../docs/test-plan.md).
