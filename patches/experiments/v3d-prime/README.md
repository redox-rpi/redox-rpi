# Experiment v3d-prime

Pi 400 graphics G4 step 2 (#670), a spike: the V3D renders straight into a
buffer of the display driver, with no copy by the CPU. Test only. **Do not
merge until the RFC decision**: buffer sharing between two drivers is a
design question for the maintainer (design note in #670).

How a frame travels:

1. A client creates a dumb buffer on its display handle (vc4d, through
   driver-graphics) and exports it with `PRIME_HANDLE_TO_FD`. The fd names
   the buffer; it carries no address.
2. The client gives the fd to Mesa (`eglCreateImageKHR` with
   `EGL_LINUX_DMA_BUF_EXT`, linear modifier). Mesa's v3d driver calls
   `PRIME_FD_TO_HANDLE` on the render node.
3. relibc (`0027`) sees the kernel refuse the two-fd call across schemes
   (`EXDEV`) and sends the fd to v3dd instead (`call` with
   `CallFlags::FD`). v3dd checks that the fd is a vc4d buffer (below),
   maps the buffer, looks up its physical pages and
   maps them into the client's own GPU address space. The answer is a BO
   handle, valid only on that client's render node file.
4. The GPU draws into the BO; the client shows the buffer with `SET_CRTC`.

Parts:

- `recipes/other/vc4d/recipe.toml`: vc4d branch `g4-prime`. vc4d moves
  to upstream base `59a2abbe` (driver-graphics with PRIME export); v3dd
  takes fds (`on_sendfd`) and reports `DRM_CAP_PRIME` with the import
  bit.
- `relibc/0027`: `PRIME_FD_TO_HANDLE` of another driver's buffer sends the
  fd to the device.
- `prime-probe` (tools/prime-probe): opens a VT of its own, renders a
  rotating cube into two vc4d buffers in turn and shows them. It checks
  that the CPU's view of the buffer (vc4d's memory) matches what the GPU
  drew, pixel for pixel. `--show` puts the VT on the screen, `--lifetime`
  destroys the buffers on the display side while the GPU still has them,
  `--foreign` offers v3dd two fds that are no display buffers (a regular
  file's and the display handle's own) and fails if v3dd takes either.

The buffer check: `fpath` and `fstat` of a received fd come from the
scheme that owns it, so they prove nothing (and driver-graphics' buffer
handles have neither). v3dd opens `/scheme/display.vc4` itself and asks it
to import the fd (`PRIME_FD_TO_HANDLE` with both fds in one call). The
kernel refuses a call on fds of two schemes (`EXDEV`), and driver-graphics
refuses an fd that is not one of its exported buffers (`EINVAL`). Only
then does v3dd map the fd; anything else answers `EINVAL`. The handle
vc4d gives back goes when v3dd closes its own fd right after.

Build with `--experiment vc4d --experiment v3dd --experiment v3d-ioctl
--experiment mesa-v3d ... --experiment v3d-prime`.
