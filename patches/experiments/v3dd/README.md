# Experiment v3dd

V3D step 3 on the Pi 4 and 400 (#598): vc4d `v3dd` starts at boot. It
powers V3D up, keeps the V3D MMU page table, hands out buffer objects and
runs compute (CSD) jobs for clients of `/scheme/v3d`. Check with:

```
v3dprobe --via-v3dd --loops 100
```

Do not run `v3dprobe --csd` or `--render` while v3dd runs: they program
their own page table and take V3D away from v3dd.

```
scripts/build-image.sh --experiment pi400 --experiment v3d-power --experiment vc4d --experiment vc4d-flip --experiment v3dd ...
```
