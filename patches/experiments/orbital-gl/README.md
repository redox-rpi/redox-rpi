# Experiment orbital-gl

Pi 400 graphics G4 step 3 (#670): Orbital composites its screen with
OpenGL ES on the V3D instead of the CPU. Test only (`test-only`), like
v3dd and v3d-prime, which it needs.

Measured first with `prime-probe --compose` (pi-tests Test 647). At 1080p
the V3D composites four half-screen windows in 5.7 ms, the CPU in 14.6 ms.
At 640x480 both take about 2 ms.

How Orbital draws with it (`orbital-gl.patch`, new `src/compositor/gl.rs`):

- At start Orbital opens EGL on the surfaceless platform with an OpenGL ES 3
  context. Each display's dumb buffer is exported as a PRIME fd and
  imported as a render target (linear ARGB8888), as in experiment
  v3d-prime. The GPU draws into the buffer vc4d shows.
- Every draw goes through `Display::rect` and the new `Display::image`. They
  draw with GL or, without it, with the CPU as before. All of Orbital's
  draws into the screen use them: background, titles, icons, window
  contents, popups, the FPS overlay and the software cursor.
- Window contents, titles and icons keep a texture each, keyed by the
  image's memory. It is uploaded once, then only where it changed: the
  damage a client reports with sync, a new title, a resize. Popups, the
  FPS overlay and the cursor go through a scratch texture.
- Before Orbital tells the display what changed (dirty framebuffer), it
  waits for the GPU (`glFinish`).
- If any step fails (no EGL, no dma-buf import, a display buffer with a
  shadow copy), Orbital logs why and draws with the CPU. `ORBITAL_GL=0`
  forces the CPU.

Build with `--experiment vc4d --experiment v3dd --experiment v3d-ioctl
--experiment mesa-v3d --experiment v3d-prime --experiment orbital ...
--experiment orbital-gl`.
