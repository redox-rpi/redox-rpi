# Experiment mesa-v3d: OpenGL ES through Mesa's v3d driver (#669)

Pi 400 graphics G3 (#669). The upstream `mesa` recipe with the `v3d`
Gallium driver (V3D 4.2, Pi 4 and 400) next to `swrast`, built as shared
libraries with EGL, plus `gles-probe` (`tools/gles-probe`).

Mesa's v3d driver talks DRM: it finds the render node
`/dev/dri/renderD128`, asks its driver name (`v3d`) and sends Linux's V3D
and syncobj ioctls. On Redox these reach `v3dd` (vc4d) through relibc, so
this experiment needs experiments `v3dd` and `v3d-ioctl`.

Three source changes in the recipe script (they would become patches in a
real port):

1. `include/drm-uapi/drm.h` includes `sys/ioctl.h` on Redox, where the
   `_IOWR` macros are; Mesa's fork leaves out `sys/ioccom.h` there and
   includes nothing instead.
2. `v3d_screen.c` takes the RAM size from Mesa's
   `os_get_total_physical_memory()` instead of `sys/sysinfo.h`, which
   relibc does not have.
3. `v3d_screen.c` returns no disk cache when the shader cache is off; Mesa
   24.0 reads `screen->disk_cache` there unguarded.

`gles-probe [frames]` draws a red triangle over a blue 64x64 framebuffer
object each frame, reads the pixels back and checks them; it prints the
renderer, a hash of the first frame and the time per frame.
`LIBGL_ALWAYS_SOFTWARE=1 gles-probe` gives the software renderer's hash
for comparison.

```bash
scripts/build-image.sh --experiment pi400 --experiment v3d-power --experiment vc4d --experiment v3dd --experiment v3d-ioctl --experiment mesa-v3d ...
```
