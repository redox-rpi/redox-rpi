# Experiment v3dd

The V3D daemon on the Pi 4 and 400 (#598, #666, #668): vc4d's `v3dd`
starts at boot. It powers V3D up, keeps the V3D MMU page table, hands out
buffer objects (BOs, which clients can also map with fmap) and runs
compute (CSD) and bin/render jobs for clients of `/scheme/v3d`. MMU
faults and jobs that do not end come back as errors; a job that hangs is
stopped by a V3D reset, and the daemon goes on. The protocol is described
at the top of vc4d's `src/bin/v3dd.rs`.

vc4d is GPL-2.0-only and lives in its own repository. The experiment
pins it in `recipes/other/vc4d/recipe.toml` (the same recipe as
experiment vc4d, so both can be used together; they should pin the same
commit) and installs the package, which also holds `v3dprobe`. Only
`v3dd` starts; vc4d's display driver starts only with experiment vc4d.
It needs experiment v3d-power for the power domain. Test only: no V3D
client uses it yet.

```
scripts/build-image.sh --experiment pi400 --experiment v3d-power --experiment v3dd ...
```

Check, as root:

```
v3dprobe --via-v3dd --loops 100                      # compute jobs
v3dprobe --via-v3dd --render --triangle --loops 100  # bin and render jobs
v3dprobe --via-v3dd --mapped --faults                # fmap, MMU fault, endless job
```

Do not run `v3dprobe --csd` or `--render` without `--via-v3dd` while
v3dd runs: they program their own page table and take V3D away from
v3dd.

v3dd arms the SoC watchdog around the power-up and around each job: the
Pi resets itself if a GPU access hangs the bus.

## Pi tests (Pi 400)

| Test | What | Result |
|---|---|---|
| 584 | compute jobs over /scheme/v3d | pass |
| 594 | bin and render jobs | pass |
| 600, 601 | MMU fault, endless job, V3D reset | pass |
| 603 | waiting on the V3D interrupt | pass |
| 604 | fmap of BOs | pass |
| 625 (rerun) | job queue (vc4d PR #24, not yet in the pinned commit) | pass |
