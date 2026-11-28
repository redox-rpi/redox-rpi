# Experiment vc4d: the VideoCore display driver instead of rpi-fbd

[#667](https://github.com/redox-rpi/redox-rpi/issues/667) (G1 of the Pi 400
graphics plan) and redox-rpi/vc4d#8. `vc4d` drives the HVS (hardware video
scaler) itself, with a primary plane, overlay planes and a hardware cursor
plane. It replaces `rpi-fbd`, which copies frames into the firmware's
framebuffer. It runs on the Pi 4 and 400 (HVS5) and on the Pi 3, 3B+ and
Zero 2 W (HVS4), and keeps the HDMI mode the firmware set at power-on.

`vc4d` is GPL-2.0-only and lives in its own repository,
[redox-rpi/vc4d](https://github.com/redox-rpi/vc4d); what it does is in its
README. This experiment holds no vc4d source: only a recipe that builds a
pinned commit, the start file, and a link to experiment `rpi-fb`'s kernel
patch for large contiguous buffers (`kernel/`).

```bash
scripts/build-image.sh --experiment pi400 --experiment vc4d \
    --experiment orbital --experiment orbital-apps --experiment desktop
```

Not together with `--experiment rpi-fb` (`conflicts`): `rpi-fbd` would take
the display first. Without it, `vesad` shows the boot framebuffer until
`vc4d` starts (`/usr/lib/init.d/15_vc4d.service`, before Orbital).

On the Pi 4 and 400, vc4d shows a waiting flip on pixel valve 2's vertical
blank interrupt instead of a 4 ms timer (`vc4d: vblank:` lines count them),
and `vc4d: irq probe:` lines show which display interrupts the firmware
uses. Both need the pi400 kernel patch that ends a GIC interrupt for user
space on the CPU that took it (PR #382). The Pi 400 needs a screen or
`hdmi_force_hotplug=1` in `config.txt`; without HDMI the firmware sets up
no HVS channel and vc4d leaves the display alone.

The base patch
[`0001`](base/0001-rpi-firmwared-raw-property-file-for-other-drivers.patch)
gives `rpi-firmwared` a `property` file (root only, not listed): a driver
writes a whole property message and reads back the firmware's answer, so
only `rpi-firmwared` uses the mailbox. vc4d reads the firmware's HDMI clock
rates through it before it changes the mode on the Pi 4 and 400 ([#671](https://github.com/redox-rpi/redox-rpi/issues/671)
step 4, redox-rpi/vc4d#36). It is the interface of experiment `rpi-fb`'s
patch `0060`, written for the current driver; the two experiments do not
go together.

Settings (environment from the bootloader): `VC4D=off` (vc4d does not
start), `VC4D_FLIP=off` (no flips, direct scanout), `VC4D_CHECK=off` (no
check before the handover), `VC4D_IRQ_PROBE=off`, `VC4D_VBLANK=off`
(the timer instead of the vertical blank), `VC4D_LOG=1` (every
display list in the log).

`desktop-check.py` checks the console and Orbital on vc4d without a person,
in QEMU or on a Pi over devsh.
