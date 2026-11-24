# Experiment mesa-v3d: Mesa's v3d Gallium driver in the build (spike, #669)

Build spike for G3 (#669): the upstream `mesa` recipe with the `v3d`
Gallium driver (V3D 4.2, Pi 4 and 400) added next to `swrast`. Only the
question whether it compiles against relibc and libdrm; nothing uses it
yet, and its DRM winsys has no kernel driver to talk to on Redox.

```bash
scripts/build-image.sh --experiment mesa-v3d r.mesa
```
