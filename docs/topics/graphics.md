# Graphics

An effort analysis, written 2026-10-29 before any work started
([#33](https://github.com/redox-rpi/redox-rpi/issues/33)).
No release target. Sizes are rough guesses for this project's way of
working (agent sessions, one Pi on the test network, a person for screen
checks).

## State

The screen works as a plain framebuffer. Nobody in Redox drives the display
hardware:

1. The Pi firmware reads the monitor's EDID at power-on, picks a mode and
   sets up HDMI.
2. U-Boot asks the firmware for a framebuffer through the mailbox and hands
   it on as a UEFI GOP display.
3. The Redox bootloader picks the "best resolution" of that GOP display
   (the menu that waits for Enter) and puts `FRAMEBUFFER_ADDR`, `_WIDTH`,
   `_HEIGHT` and `_STRIDE` into the kernel's environment.
4. `vesad` maps that memory and offers it as a display; `fbcond` draws the
   text console on it, `fbbootlogd` the boot log.

What this means:

- **Resolution:** whatever the firmware chose at power-on. On the test Pi
  that is 1920x1200, 32 bits per pixel (all but one of about 60 boots in
  the serial logs of the last two weeks). It cannot change while Redox runs.
- **Once 640x480:** one boot on 2026-10-29, after a night of watchdog
  resets, came up at 640x480. The monitor probably did not answer the
  firmware at that moment. Not checked.
- **HDMI hotplug:** none. A monitor plugged in after power-on gets the mode
  of the boot, or no picture if there was none. Nothing in Redox notices.
- **Without a monitor:** not tested yet. The firmware then normally falls
  back to composite video or a default mode, and the boot goes on (the
  network tests run headless). To check with `hdmi_force_hotplug` off and
  no cable.
- **Direct boot (#15):** `rpi-boot-stub` sets up no framebuffer, so `vesad`
  prints `No boot framebuffer` and there is no picture. Only the serial
  console works there.
- **Speed:** all drawing is done by the CPU in software, into memory the
  firmware set aside. `gpu_mem=16` in `config.txt` leaves the VideoCore
  just 16 MiB; enough for a 1920x1200 framebuffer (9 MiB), not for 3D.

## Upstream Redox

Redox has a real graphics driver interface: `driver-graphics`
speaks the **Linux DRM/KMS interface** (the ioctls `MODE_GET_CONNECTOR`,
`MODE_SET_CRTC`, `MODE_CREATE_DUMB`, `MODE_DIRTYFB`, planes, cursor, gamma,
hotplug events). Display servers use it through `graphics-ipc`, the same way
Linux programs use `/dev/dri/card0`.

| Driver | What | Size |
|---|---|---|
| `vesad` | the boot framebuffer, one fixed mode | 400 lines |
| `virtio-gpud` | QEMU's virtio GPU, 2D only (modes, EDID, hotplug) | 1200 lines |
| `ihdgd` | Intel display engine: modes, outputs, no 3D | 4800 lines |
| `driver-graphics` | the shared DRM/KMS layer | 2900 lines |

There is **no 3D support** anywhere in Redox: no render nodes, no GEM
buffer sharing, no way to submit GPU commands. Mesa runs on Redox only as
the software renderer (llvmpipe/OSMesa); Orbital, the desktop, composites
on the CPU into a dumb buffer.

## Step 1: a firmware framebuffer driver

A new driver (or a second scheme inside `rpi-firmwared`, which already owns
the mailbox) that sets up the framebuffer itself through the firmware's
property tags, instead of inheriting U-Boot's:

| Tag | Use |
|---|---|
| `0x00030020` get EDID block | the monitor's modes, block by block |
| `0x00048003` / `0x00048004` set physical / virtual size | the resolution |
| `0x00048005` set depth, `0x00048006` pixel order | 32 bits, RGB or BGR |
| `0x00040001` allocate buffer, `0x00040008` get pitch | the buffer and its line length |
| `0x00048009` set virtual offset | double buffering by moving the visible window |
| `0x00040002` blank screen | screen off and on |

It implements `GraphicsAdapter` from `driver-graphics`, like `vesad`, but
with a connector whose modes come from the EDID and a `set_crtc` that asks
the firmware for a new size. `fbcond` and Orbital work on it unchanged.

- **What it gives:** the resolution can be chosen and changed at runtime;
  the EDID is readable; double buffering without tearing; the screen can be
  blanked. It works without U-Boot, so **#15 direct boot gets a picture**.
- **What it does not give:** real hotplug. The firmware sends no event; the
  driver could only poll the EDID every few seconds and re-allocate when a
  monitor appears. The firmware still owns the display hardware.
- **Size: 1 to 2 weeks**, including Pi tests with a person at the screen.
  About 600 to 900 lines, most of it like `vesad` and `virtio-gpud`.
- **Smaller first step for #15: 1 to 2 days.** `rpi-boot-stub` asks the
  firmware for a framebuffer (the same tags, before the kernel starts) and
  writes the `FRAMEBUFFER_*` variables like the Redox bootloader does.
  `vesad` stays as it is. Fixed resolution, but direct boot has a screen.

## Step 2: faster 2D

The CPU copies every changed pixel today. Two ways to take work off it:

- **DMA blits: 2 to 4 weeks**, after the common DMA plan (#21). The BCM2835
  DMA engine copies and fills 2D rectangles. Useful for `MODE_DIRTYFB`
  copies, scrolling in `fbcond` and Orbital's damage copies. Gain: maybe
  half of the copy time and a free CPU core; the memory bandwidth stays
  the same. Worth measuring the CPU cost first (a full 1920x1200 frame is
  9 MiB).
- **Taking the display over from the firmware: 2 to 3 months.** Linux's
  `vc4` KMS driver programs the display hardware itself: the HVS (hardware
  video scaler, which composes several planes with scaling), the pixel
  valves (timing) and the HDMI encoder with its clocks and hotplug pin.
  That brings real hotplug, hardware planes and a hardware cursor, and
  frees the screen from the firmware's choices. It is the part of the Linux
  driver that has the most code and the most Pi-model differences; the
  firmware path of step 1 covers most needs.

## Step 3: 3D with the V3D GPU

The VideoCore IV's 3D block (V3D) does OpenGL ES 2.0 (no Vulkan; Mesa's
`v3dv` is for the Pi 4 and later). Broadcom published its register and
architecture guide in 2014, and Linux and Mesa have open drivers:

- **Kernel side (Linux `vc4`):** allocates GPU buffers as physically
  contiguous memory (CMA), because the V3D has **no MMU**; takes command
  lists for the binner (sorts triangles into tiles) and writes the render
  command lists itself; **checks every command list and shader** before it
  runs, because without an MMU a bad one could read or write any memory;
  waits for the GPU's interrupts and keeps fences.
- **User side (Mesa `vc4`):** a Gallium driver that compiles shaders and
  builds command lists, talking to the kernel through `vc4` ioctls of
  libdrm, plus EGL and GBM to bring buffers to the screen.

What Redox would need:

1. A V3D driver daemon with the `vc4` ioctls (create buffer, map, submit,
   wait), including the command list and shader checks, ported from Linux:
   the checks are the hard and security-critical part.
2. Extensions to `driver-graphics`: render nodes, GEM handles and buffer
   sharing between processes (PRIME), sync objects.
3. Mesa's `vc4` driver built for Redox, with a Redox platform for EGL/GBM.
4. Orbital (or a newer compositor) using EGL instead of copying in software.
5. Much more GPU memory: `gpu_mem` and a CMA-like pool, taken from the
   1 GiB (512 MiB on the Zero 2 W).

- **Size: 6 months or more**, and it touches upstream Redox interfaces that
  do not exist yet (no other Redox driver needs render nodes so far).
- **Benefit on a Pi 3: small.** GLES 2.0 with little memory; the Redox
  desktop is 2D. It makes sense only once upstream Redox has a 3D path for
  some other GPU that this could follow.
- **The other way, the firmware's own GL (VCHIQ): not recommended.**
  Circle and Ultibo get OpenGL ES 1.1/2.0 and OpenVG by porting the Pi's old
  closed "userland" libraries, which hand the GL calls to a driver running
  on the VideoCore. It needs VCHIQ (on the wish list, large) and the full
  firmware (`gpu_mem` above 16, not the cut-down `start_cd.elf`). Raspberry
  Pi itself dropped that driver, and in Circle it works only for 32-bit
  ARM.

## Other systems on the Pi

| System | Display | 3D |
|---|---|---|
| Linux | `vc4` KMS driver (step 2), formerly firmware KMS | Mesa `vc4` (step 3) |
| Circle (C++ bare metal) | firmware framebuffer (step 1) | firmware GL over VCHIQ, 32-bit only |
| Ultibo (Pascal) | firmware framebuffer | firmware GL over VCHIQ |
| NetBSD | firmware framebuffer | firmware GL over VCHIQ (pkgsrc) |
| FreeBSD, Plan 9 | firmware framebuffer | none |

Every hobby system stopped at step 1 or took the closed firmware GL path.
Only Linux has an open display and 3D driver, written with Broadcom's help.

## Next

- Step 1 small version for #15: framebuffer from `rpi-boot-stub`.
- Check the boot without a monitor, and with a monitor plugged in later.

## Later

- Step 1 full driver, then DMA blits once #21 is done.
- Steps 2 (own display driver) and 3 (V3D) only if upstream Redox grows a
  3D path, or if someone wants to do it as a project of its own.

## Sources

- [Mailbox property interface](https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface) (framebuffer and EDID tags)
- [drm/vc4 Broadcom VC4 Graphics Driver](https://docs.kernel.org/gpu/vc4.html) (no MMU, CMA, command list and shader checks)
- [vc4_validate.c](https://lxr.missinglinkelectronics.com/linux+v4.16/drivers/gpu/drm/vc4/vc4_validate.c)
- [Circle VC4 subsystem](https://circle-rpi.readthedocs.io/en/latest/subsystems/vc4.html), [Ultibo VC4 unit](https://ultibo.org/wiki/Unit_VC4), [NetBSD vchiq(4)](https://man.netbsd.org/evbarm/vchiq.4)
- Upstream Redox `base`: `drivers/graphics/` (sizes counted 2026-10-29)
