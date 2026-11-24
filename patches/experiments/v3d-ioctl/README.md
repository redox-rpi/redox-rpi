# Experiment v3d-ioctl

Pi 400 graphics G3 (#669): relibc passes the V3D driver ioctls on, so
Mesa's v3d driver can reach `v3dd` (experiment v3dd) through libdrm.

Two relibc patches:

`0024`:

- `DRM_IOCTL_GEM_CLOSE` (0x09) and Linux's V3D ioctls `SUBMIT_CL`,
  `WAIT_BO`, `CREATE_BO`, `MMAP_BO`, `GET_PARAM`, `GET_BO_OFFSET` and
  `SUBMIT_CSD` (DRM_COMMAND_BASE 0x40 + nr) become scheme calls with the
  function number as metadata, like relibc's other DRM ioctls. The
  structs follow Linux's `include/uapi/drm/v3d_drm.h`; a submit's BO
  handles travel inline after the fixed part (`redox-ioctl`'s wire form).
- Before the patch these requests answer EINVAL without reaching the
  driver. Function numbers from 0x40 up are driver-specific, which is why
  this is an experiment and not a default patch: on another DRM device the
  same numbers would mean something else.

`0025`: DRM sync objects, `SYNCOBJ_CREATE`, `DESTROY`, `WAIT`, `RESET`
and `SIGNAL` (0xBF, 0xC0, 0xC3 to 0xC5). Mesa's v3d driver fences its
jobs with them (libdrm `drmSyncobj*`); v3dd keeps them per file.

The numbers start at 0024, after the default relibc patches 0020 to 0023 of
open pull requests; the `exp-v3d-ioctl-` prefix keeps them apart in any case.

Test: `v3dprobe --via-v3dd --ioctl --libc` (vc4d) runs v3dd's binary
interface check through relibc's `ioctl()` with Linux's request numbers
and structs, the path libdrm takes. Without this experiment every call
there answers EINVAL.

```
scripts/build-image.sh --experiment pi400 --experiment v3d-power --experiment vc4d --experiment v3dd --experiment v3d-ioctl ...
```
