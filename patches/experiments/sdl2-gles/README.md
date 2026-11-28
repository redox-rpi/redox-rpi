# Experiment sdl2-gles: SDL2 programs with OpenGL ES on the V3D (#669)

Pi 400 graphics G3 (#669). SDL2's Orbital video driver could draw OpenGL
only through OSMesa (software), and experiment orbital-apps builds SDL2
without OpenGL at all. This experiment replaces the sdl2 recipe with
orbital-apps' version plus OpenGL ES through EGL:

- `SDL_orbitalopengl.c` is rewritten for EGL's surfaceless platform (as
  `gles-probe` and `gles-demo` in experiment mesa-v3d). Each context draws
  into an EGL pbuffer of the window's size, so framebuffer 0 works as
  programs expect. `SDL_GL_SwapWindow` reads the pbuffer back and copies
  it into the Orbital window. `SDL_GL_GetProcAddress` is
  `eglGetProcAddress`. OpenGL ES 2.0 and 3.x contexts, and desktop OpenGL
  if a program asks for it and the driver has it.
- Windows without OpenGL keep orbital-apps' framebuffer path.
- `ORBITAL_DestroyWindow` no longer frees the window's GL context: the
  program does that with `SDL_GL_DeleteContext`, also after
  `SDL_DestroyWindow`, which freed it twice.
- SDL is built with `-DSDL_OPENGLES=ON` and links EGL instead of OSMesa.

- SDL's 2D renderer defaults to `software` on Orbital (`SDL_HINT_RENDER_DRIVER`
  at default priority, set in `ORBITAL_VideoInit`). Its OpenGL ES 2.0 backend
  would draw on the V3D, then read every frame back with the CPU, which made
  Sopwith slow (#769). `SDL_RENDER_DRIVER=opengles2` or a program's own hint
  still picks it. `SDL_GL_*` contexts are not affected.

Needs experiments `v3dd`, `v3d-ioctl`, `scanf-pipe`, `mesa-v3d` and
`orbital-apps`. **List it after `orbital-apps`:** both replace the sdl2
recipe, and the last one listed wins.

`sdl-gles-probe [frames]` checks an OpenGL ES 2.0 context through SDL_GL:
a triangle in a colour that cycles every 30 frames, read back and checked
exactly, then `SDL_GL_SwapWindow`. `sdl-gles-probe --renderer [frames]`
checks SDL's renderer with a red square; it asks for `opengles2` unless
`SDL_RENDER_DRIVER` is set. Both print the renderer, a hash
of the first frame and the time per frame. `LIBGL_ALWAYS_SOFTWARE=1` gives
the software renderer's hash for comparison.

```bash
scripts/build-image.sh ... --experiment v3dd --experiment v3d-ioctl --experiment scanf-pipe \
    --experiment mesa-v3d --experiment orbital --experiment orbital-apps --experiment sdl2-gles ...
```
