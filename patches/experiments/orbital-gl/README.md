# Experiment orbital-gl

Pi 400 graphics G4 step 3 (#670): Orbital composites its screen with
OpenGL ES on the V3D instead of the CPU. Test only (`test-only`), like
v3dd and v3d-prime, which it needs.

Measured first with `prime-probe --compose` (pi-tests Test 647). At 1080p
the V3D composites four half-screen windows in 5.7 ms, the CPU in 14.6 ms.
At 640x480 both take about 2 ms. In Orbital itself (Test 649, 1080p) a
window drag costs Orbital 17 % of a core on the GPU path and 81 % on the
CPU path; with v3dd and vc4d the sums are 38 % and 83 %.

How Orbital draws with it (`orbital-gl.patch`, new `src/compositor/gl.rs`):

- At start Orbital opens EGL on the surfaceless platform with an OpenGL ES 3
  context. Each display's dumb buffer is exported as a PRIME fd and
  imported as a render target (linear ARGB8888), as in experiment
  v3d-prime. The GPU draws into the buffer vc4d shows.
- Every draw goes through `Display::rect` and the new `Display::image`. They
  draw with GL or, without it, with the CPU as before. All of Orbital's
  draws into the screen use them: background, titles, icons, window
  contents, popups, the FPS overlay and the software cursor.
- Before Orbital tells the display what changed (dirty framebuffer), it
  waits for the GPU (`glFinish`).
- If any step fails (no EGL, no dma-buf import, a display buffer with a
  shadow copy), Orbital logs why and draws with the CPU. `ORBITAL_GL=0`
  forces the CPU. The log names the mode at start: "GL compositing on the
  GPU" or "GL compositing switched off".

## Textures

- A window's buffer, its shadow copy and its two title images each have a
  `gl::Cache` next to them in the `Window`: the key of its texture and
  the parts that changed since the last upload. The texture is uploaded
  once, then only where it changed: the damage a client reports with
  sync, a new title, a resize, a new mapping. Keys are never reused, so a
  new image never gets an old image's texture.
- Icons, popups, the FPS overlay and the cursor have no cache; they go
  through one scratch texture.
- Textures not drawn for 5 s are deleted (closed, hidden or covered
  windows); a window drawn again later is uploaded in full.

## Design choices

- **Clients that write without a sync.** Orbital's contract is that a
  client syncs what it changed; the CPU path shows such writes only when
  something else redraws that part, and not at all for transparent windows
  (they draw from the shadow copy, which only a sync updates). The GPU path
  uploads a window's buffer in full once a second while the window is
  drawn, so such writes appear with at most that delay. During a drag of a
  half-screen window at 1080p that is about 4 MB a second.
- **One thread.** Orbital has a single thread, and the GL state stays on
  it: `gl::Cache` uses cells (not `Sync`) and the renderer holds raw EGL
  pointers (not `Send`), so the compiler refuses use from another thread.
  There is no global or thread-local state.
- **Other Orbital experiments.** orbital-planes (#583) also replaces
  `recipes/gui/orbital/recipe.toml`, and both change how a dragged window is
  drawn, so the two do not go together (`conflicts`). Whichever lands
  second carries the other's patch in its recipe if both are wanted. The
  same applies when the Orbital recipe of #456 lands: orbital-gl's recipe
  then lists its patch as well.

Build with `--experiment vc4d --experiment v3dd --experiment v3d-ioctl
--experiment mesa-v3d --experiment v3d-prime --experiment orbital-gl` (plus
the desktop experiments of the image).
