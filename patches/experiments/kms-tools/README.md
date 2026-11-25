# Experiment kms-tools

Console graphics without Orbital (#728), step 2: check upstream's packaged
libdrm against vc4d's KMS ioctls. Test only (`test-only`).

- Packages `libdrm.tests` (libdrm's test programs: `modetest`, `modeprint`,
  `proptest`, `vbltest`, `drmdevice`) and `drm-info`, both upstream
  recipes as they are.
- `redox-open.patch` for libdrm:
  - `drmOpenDevice` opens the node and nothing else on Redox. Upstream
    libdrm creates or replaces a node it cannot open or stat with
    `mknod`, which made empty regular files `card1`–`card63` in
    `/dev/dri` and, while vc4d was not up, replaced the `card0` link
    (Test 652).
  - The test programs' `-D` takes a device path (`/dev/dri/card0`) and
    opens it directly. Upstream treats `-D` as a bus id for `drmOpen`,
    which needs bus ids that Redox drivers do not have.
- `/dev/dri/card0` links to `/scheme/display.vc4/7`, where libdrm looks
  for the primary node. driver-graphics serves one set of KMS state per
  virtual terminal, named by the path; 7 is a VT nothing else uses (the
  console is on 2, Orbital on 3). What a program sets there shows only
  while VT 7 is active (`inputd -A 7`; `inputd -A 2` brings the console
  back).

Build with `--experiment pi400 --experiment vc4d --experiment kms-tools (vc4d is not on main yet: it comes with the G3/G4 branches)`.

Test: `drm_info`, `modetest -D /dev/dri/card0` (lists connectors, CRTCs,
planes and properties), and `modetest -D /dev/dri/card0 -s <connector>:<mode>`
(modeset with a dumb buffer test pattern) while VT 7 is active.
