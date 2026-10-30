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
- **Direct boot (#15):** `rpi-boot-stub` on `fw-direct-boot` sets up no
  framebuffer, so `vesad` prints `No boot framebuffer` and there is no
  picture. Only the serial console works there (fixed on `graphics-fb`,
  see In progress).
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

## Step plan at a glance

From the firmware's framebuffer to the GPU, each step on top of the one
before. "Before" is the estimate written before any code, "Now" what
QEMU showed by 2026-10-30 (details in the step sections, In progress and
Effort below).

| Step | What | Before | Now | Left, new estimate |
|---|---|---|---|---|
| 1a | Firmware framebuffer: `rpi-fbd` gets its buffer, modes and EDID through the mailbox (`rpi-firmwared`), boot stub framebuffer for direct boot | 1–2 weeks | works in QEMU (`graphics-fb`) | Pi test #34; a few days of calendar time |
| 1b | Double buffering by page flip: two screens in one buffer, the mailbox's virtual offset (tag `0x00048009`) shows the finished one; EDID polling, blanking | part of 1a | works in QEMU (`graphics-fb2`, patch `0026`); waiting for the vertical blank after a flip on `graphics-vsync` (patch `0036`, tag `0x0004800e`, falls back where the firmware lacks it, as in QEMU) | Pi tests #43 and #97. Missing: two screens at 1920x1080/1200 (need `gpu_mem` 32 or more, two 8–9 MiB screens); 1–3 days with Pi tests |
| 2a | DMA blits: the copy to the screen by the DMA controller | 2–4 weeks | prototype in QEMU (`graphics-fb3`) | Pi test #44 decides if it pays; cached dumb buffers need cache maintenance from user space (kernel, DMA plan #21); a few days |
| – | Desktop: Orbital on `rpi-fbd`, keyboard, terminal, mouse | not planned | works in QEMU (`graphics-gui`, `graphics-gui-mouse`) | Pi test #54 and the mouse test; faster terminal is upstream work |
| 2b | Own display driver (VC4 HVS, pixel valves, HDMI): real hotplug, hardware planes and cursor, modes without the firmware | 2–3 months | not started | unchanged; no template in Redox, debugging on the Pi with someone at the screen (the NanoKVM helps) |
| 3 | 3D with the V3D (OpenGL ES 2.0 through Mesa `vc4`) | 6+ months | not started | unchanged; needs render nodes and buffer sharing in upstream Redox first |

Steps 1 and 2a went fast because Redox had a template for every part and
the firmware does the HDMI work; 2b and 3 have neither, so their
estimates stand.

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

## In progress

Branch [`graphics-fb`](https://github.com/redox-rpi/redox-rpi/tree/graphics-fb)
(on `fw-direct-boot`), started 2026-10-29:

- **Boot stub framebuffer (small step, done in QEMU):** `rpi-boot-stub`
  asks the firmware for a 32-bit framebuffer of the display size it chose
  (same pixel order and alpha mode as U-Boot) and passes it in the
  `FRAMEBUFFER_*` variables. `vesad`, `fbcond` and the kernel's boot
  messages use it unchanged, so direct boot (#15) has a screen.
  `RPI_FB=1280x720` in `cmdline.txt` picks another size, `RPI_FB=off`
  none. QEMU: login prompt on screen at 640x480 and 1280x720.
- **`rpi-fbd`, step 1 driver (first version, works in QEMU):** experiment
  [`rpi-fb`](https://github.com/redox-rpi/redox-rpi/tree/graphics-fb/patches/experiments/rpi-fb).
  It talks to the firmware through a new `property` file of
  `rpi-firmwared` (only one process uses the mailbox), reads the EDID,
  offers the display size, the EDID modes and common sizes, and takes the
  console over from `vesad`. A mode change allocates a new framebuffer;
  the firmware scales it to the monitor's mode from power-on. QEMU:
  handover and a switch from 640x480 to 1280x720 at runtime work.
- **Without a monitor:** the display counts as connected at the firmware's
  fallback size, `rpi-fbd` logs `no EDID`, and the boot goes on.
- **Monitor turned on after power-on (user finding 2026-10-29):** no
  picture. The firmware switches HDMI on only when a monitor answers at
  power-on, and nothing in Redox can switch it on later: the documented
  property interface has no HDMI or hotplug tag, only the EDID. Fix on the
  branch: `hdmi_force_hotplug=1` in `config/config.txt` (netboot and SD
  images), so HDMI is always on; a monitor on at power-on still gets its
  own mode, one turned on later gets the firmware's fallback mode. A fixed
  mode (`hdmi_group`/`hdmi_mode`, commented out there) would give every
  monitor that size, but loses the native 1920x1200 of the test monitor.
  Pi check in #34. Real hotplug (new mode for a new monitor) needs the
  display hardware itself (step 2); `rpi-fbd` could at most notice a new
  monitor by polling the EDID.
- **Pi test #34** in the [queue](../test-plan.md), image built. The screen
  checks need a person and wait for another visit.
- **Branch [`graphics-fb2`](https://github.com/redox-rpi/redox-rpi/tree/graphics-fb2)**
  (on `graphics-fb`, so #34 stays as queued), patch `0026`, works in QEMU
  (`v0.3.0-dev-179-gc7d38dd+xb2e2`, homenodellm
  `~/redox-rpi-graphics/keep/`):
  - **EDID polling** every 5 seconds: a monitor that comes or goes is
    logged and clients get a hotplug event. It does not give the monitor a
    new HDMI mode (the firmware keeps the one from power-on), and whether
    the firmware reads the EDID again later is only known after a Pi test.
  - **Double buffering** with the firmware's virtual offset, when two
    screens fit into half of the GPU memory: with `gpu_mem=16` up to about
    1280x800, so **not at the test monitor's 1920x1200** (that needs
    `gpu_mem=32`, 16 MiB less RAM for Redox).
  - **Screen blanking** when a client sets no mode; nothing in Redox does
    that yet.
- **Branch [`graphics-fb3`](https://github.com/redox-rpi/redox-rpi/tree/graphics-fb3)**
  (on `graphics-fb2`), patch `0027`, **step 2 prototype: DMA blits**, works
  in QEMU (`v0.3.0-dev-189-gb7fe574+xb2e2`). Built on the DMA plan of #21:
  channel 5, one 2D control block per frame, dumb buffers in uncached DMA
  memory, the register code copied from `bcm2835-sdhostd` (to move into a
  common crate later, agreed with the SD session). A test blit at start
  finds out whether the controller moves YLENGTH rows or one more (QEMU:
  one more). `RPI_FB_DMA=off` copies with the CPU, `RPI_FB_BENCH=1`
  measures drawing and blits both ways. QEMU cannot say whether it pays
  off: uncached dumb buffers make drawing slower, DMA makes the blit
  cheaper; Pi test #44.
- **Branch [`graphics-gui`](https://github.com/redox-rpi/redox-rpi/tree/graphics-gui)**
  (on `graphics-fb3`), **effort probe: Orbital, Redox's desktop**, works in
  QEMU at 1280x720 (`v0.3.0-dev-193-g2983d13+xdf3e`, experiment
  [`orbital`](https://github.com/redox-rpi/redox-rpi/tree/graphics-gui/patches/experiments/orbital)):
  - **It runs unchanged.** Orbital, its login window, launcher and
    terminal come as binary packages (upstream's `desktop-minimal`
    config); `rpi-fbd` is a normal Redox display to it, with DMA blits.
  - **Two `rpi-fbd` fixes it showed:** double buffering copied the whole
    screen every frame after a while (the damage of both buffers was kept
    as a union that only grew; in `graphics-fb2` too, costs speed only),
    and Orbital composes in the buffer it gets, which with DMA is uncached
    memory, slow to read; clients now draw into a shadow buffer in normal
    memory and only copy the changed part over.
  - **Input:** the USB keyboard works (`dwc2d`, one keyboard in the boot
    protocol), but **there is no mouse driver**, so Orbital is keyboard
    only. `dwc2-interrupts` (a default patch on main) is needed; without
    it `dwc2d` polls on a whole core.
  - **Memory:** 140 MiB used without Orbital, about 160 MiB with the
    desktop, of 932 MiB (QEMU, `free`). Fits the Pi's 1 GB easily.
  - **Frame rate:** QEMU's DMA model is slow (30 to 57 frames per second
    for 512x512 in `drawbench`), so the real number comes from the Pi
    test #54. The login to desktop takes under 90 s in QEMU (earlier runs
    measured minutes, but the test helper caused that; see Mouse), to be
    timed on the Pi.
  - **Keyboard and terminal** (`v0.3.0-dev-195-g671b427+xdf3e`):
    `dwc2d`'s existing keyboard events reach Orbital and the terminal
    window without changes (`echo Hi` typed in `orbterm` ran; Super-F12
    shows Orbital's FPS counter). The launcher's Super-T asks for
    `cosmic-term`, which is not packaged; the image links it to `orbterm`.
    Without a mouse, Super-Numpad-0 turns on Orbital's mouse keys.
  - **Terminal speed:** about 4 ms per output line for drawing (`ls -R
    /usr`, 679 lines: 5.6 s in `orbterm`, 2.7–3.0 s to `/dev/null`).
    `orbterm` renders each character with `rusttype` without a glyph
    cache and sends its whole window (1024x600, 2.4 MB) on every update;
    the DMA blits are not the limit. A faster terminal is upstream work in
    `orbterm`/`orbfont` (glyph cache, only the changed rows as damage).
  - **Memory with a terminal:** 194 MiB used (login window plus
    `orbterm`); the biggest part is the decoded background picture (84
    MB in `background`).
  - **Mouse** (branch `graphics-gui-mouse`, this work merged with the
    USB session's `usb-mouse`, #48; image
    `v0.3.0-dev-222-g67cef7a+xd99d`): in QEMU the pointer moves, hovering
    shows the launcher's tooltips and the buttons arrive, with no change
    in Orbital or `rpi-fbd`. **The whole desktop works by keyboard and
    mouse** (2026-10-30): Enter at the login window, the desktop within 90
    s, a click on the launcher's terminal icon opens `orbterm` within 90
    s, and `echo hi` typed there prints `hi`. Earlier runs showed late
    moves, logins of minutes, a click that opened nothing and `getty` and
    `bcm2835-sdhci` at 12 minutes of CPU; all of that came from the test
    helper, which did not read QEMU's serial output while sending input,
    so programs writing to the console blocked.
  - **Memory for the desktop with `gpu_mem=16`:** in QEMU (`free`, 932 MiB
    for Redox) 194 MiB used at the login window and 270 MiB with the
    desktop and a terminal: `background` 82 MB (the decoded picture),
    `orbital` 25 MB, `launcher` 9 MB, `orbterm` 6 MB. On the Pi,
    `gpu_mem=16` leaves Redox about 1008 MiB, more than QEMU's 932, so
    the desktop uses about a quarter of it. The framebuffer itself is in
    the GPU's 16 MiB (1280x720 double buffered 7 MiB, 1920x1080 single 8
    MiB); the clients' buffers and shadow copies are in Redox's memory
    and are part of the 270 MiB. The biggest single saving would be a
    smaller background picture.
  - **NanoKVM:** the Pi's HDMI now goes to a NanoKVM. Its EDID will set
    the display size (probably 1920x1080, then single buffered with
    `gpu_mem=16`), and its USB side can be the Pi's keyboard (and later
    mouse) for #54. Once the user has signed in, screenshots from it can
    replace the person for the screen checks of #34, #43, #44 and #54
    (see the test plan rows).

## Next

- **rpi-fb towards a PR:** Pi test #116 (serial only, Pi test agent)
  judges `rpi-fbd` from the boot log, the `RPI_FB_BENCH` numbers,
  `drawbench` and the vsync line, without a screen. If it passes, a draft
  PR from `graphics-vsync` brings experiment `rpi-fb` (patches `0024`–
  `0029`, `0036`) and experiment `orbital` (a config file) to main, still
  as experiments, not default; the report goes into the PR. Making
  `rpi-fbd` the default waits for the screen visit (#34, #43, #44, then
  #54 with #108), which checks what the screen shows.
  Known limit for the PR: at 1824x984 (the NanoKVM's mode) the Pi logged
  `dumb buffer 1824x984 in normal memory: Out of memory` with 591 MB
  free. The 7 MB DMA buffer needs physically contiguous memory, and none
  that large was left; `rpi-fbd` then copies that buffer with the CPU, as
  designed. Allocating the large buffers early or DMA through page lists
  would lift it; later. Boots 2–3 of #116 set `RPI_FB*` through
  `cmdline.txt` (patch `0037`), which needs the user's OK for the shared
  file in `/srv/tftp/`.
- Pi test #34; the stub framebuffer on the Pi with #18 (direct boot).
- Pi test #43 (`graphics-fb2`) right after #34: monitor turned on after
  boot (is `monitor connected` logged?), and double buffering at 1280x720.
- Pi test #44 (`graphics-fb3`), same visit: the DMA row count and the
  benchmark, CPU against DMA. If DMA wins, cached dumb buffers with cache
  maintenance (DMA plan step 2, a kernel change) are the next gain.
- Pi test #54 (`graphics-gui`), same visit: Orbital on the real screen,
  login to desktop time, frame rate with `drawbench`, terminal speed,
  memory. With the NanoKVM most screen checks need no person.
- For a usable desktop: the USB mouse works in QEMU (`usb-mouse` on
  `usb-hotplug`, #48, keyboard and mouse at the same time); next is the
  Pi test with a mouse (the NanoKVM's USB can be one) and then `usb-mouse`
  and the graphics branches on main once their Pi tests pass.

## Effort: estimate and what it took

The estimates above were made before any code. What step 1 really took,
in agent time on 2026-10-29 (from the commit times; one session, builds
on homenodellm about 3 to 20 minutes each, QEMU runs 2 to 5 minutes):

| Piece | Estimate | Took, to working in QEMU | Lines |
|---|---|---|---|
| Boot stub framebuffer (#15 small step) | 1–2 days | 20 minutes | 210 (stub) |
| `rpi-fbd` first version: firmware calls, EDID, mode change, handover from `vesad`; `rpi-firmwared` property file | part of 1–2 weeks | 1 hour, with a QEMU test that types into the bootloader | 650 (driver), 70 (`rpi-firmwared`) |
| Monitor turned on later: diagnosis and `config.txt` fix | not planned | 10 minutes | 14 |
| EDID polling, double buffering, blanking (`graphics-fb2`) | part of 1–2 weeks | 40 minutes, half of it build and QEMU | 220 |
| Step 2 prototype: DMA blits (`graphics-fb3`), with a benchmark | 2–4 weeks | 35 minutes, half of it three builds and QEMU | 350 (`dma.rs`, 200 of them copied from `bcm2835-sdhostd`), 240 (driver) |
| Orbital on `rpi-fbd` (`graphics-gui`): login window on screen | not estimated | 20 minutes (a config file, binary packages) | 25 (config) |
| … then input, memory, frame rate, two `rpi-fbd` fixes, `drawbench` | not estimated | 1 hour more, most of it QEMU runs (each 5 to 15 minutes, the emulated desktop is slow) | 20 (driver), 3 (Makefile) |
| … keyboard in Orbital, terminal window, its speed and memory | not estimated | 1.5 hours, almost all QEMU runs (keys get lost while the emulated desktop is busy) | 5 (config: `cosmic-term` link) |
| … mouse in Orbital (the USB session's `usb-mouse` merged in) | a few hours | 1 hour, one build and two QEMU runs; the driver was the USB session's | 0 |
| Waiting for the vertical blank after a flip (`graphics-vsync`) | part of 1–3 days | 40 minutes, with one build and a QEMU run; the tag is undocumented, so the Pi test decides | 55 |

So the code of step 1 took about **2 hours instead of 1–2 weeks**. Why
it went fast: every part had a template in Redox (`vesad` for the
adapter, `virtio-gpud` for modes and hotplug, `rpi-firmwared` for the
mailbox, the stub for the boot side), the firmware does the hard part
(HDMI timing), and QEMU emulates the framebuffer tags.

What is **not** in that time, and where the rest of the estimate goes:

- **The Pi tests.** Nothing of this has run on a real Pi yet. #34 waits
  behind the v0.3.0 tests in the queue, and its screen checks need a
  person at the Pi (the user travels). Calendar time until step 1 is
  tested: days to weeks, set by the queue and visits, not by the code.
- **What QEMU cannot show:** QEMU has no EDID tag, swaps red and blue,
  scales nothing, and never tears. The EDID on the real firmware
  (`start_cd.elf` with `gpu_mem=16`), the colours, the scaling and the
  flip timing are unknown until the Pi test; each surprise costs a
  build-and-test round (about an hour with the queue free, plus a
  person for anything on the screen).

What this says about the other steps: the time of step 1 came from
reusing templates and letting the firmware do the hardware.

The DMA blit prototype checked the first estimate of step 2. It had a
template too (the SD session's DMA code for #21, same controller), and
QEMU emulates the 2D mode, so the code took 35 minutes instead of 2–4
weeks. What is still open is not code: whether DMA pays off at all (only
the Pi's numbers say), the real controller's row count in 2D mode, and,
if it pays off, cached dumb buffers, which need cache maintenance from
user space (a kernel change, DMA plan step 2, shared with #21). New
estimate for DMA blits: **a few days** of calendar time, set by the Pi
test and that kernel change, not by the driver.

The desktop probe went the same way: Orbital needed no code, because
`rpi-fbd` speaks the same display interface as every Redox display
driver, and the hour after the first picture went into measuring and
into two `rpi-fbd` fixes that only a real client showed. What a usable
Pi desktop still needs is a mouse driver (hours) and the Pi's own
numbers (frame rate, login time), which wait for the visit.

The other two estimates stay as they are, unchecked: own display driver
2–3 months, V3D 6+ months. Neither has a template in Redox, and their
main work is debugging registers on the real Pi with someone watching
the screen.

## Later

- DMA blits beyond the prototype once #21's common DMA code and cache
  maintenance exist.
- Steps 2 (own display driver) and 3 (V3D) only if upstream Redox grows a
  3D path, or if someone wants to do it as a project of its own.

## Sources

- [Mailbox property interface](https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface) (framebuffer and EDID tags)
- [drm/vc4 Broadcom VC4 Graphics Driver](https://docs.kernel.org/gpu/vc4.html) (no MMU, CMA, command list and shader checks)
- [vc4_validate.c](https://lxr.missinglinkelectronics.com/linux+v4.16/drivers/gpu/drm/vc4/vc4_validate.c)
- [Circle VC4 subsystem](https://circle-rpi.readthedocs.io/en/latest/subsystems/vc4.html), [Ultibo VC4 unit](https://ultibo.org/wiki/Unit_VC4), [NetBSD vchiq(4)](https://man.netbsd.org/evbarm/vchiq.4)
- Upstream Redox `base`: `drivers/graphics/` (sizes counted 2026-10-29)
