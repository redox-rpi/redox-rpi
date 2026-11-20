# Experiment vc4d-trace

Turns on vc4d's trace ([#563](https://github.com/redox-rpi/redox-rpi/issues/563)):
the empty file `/etc/vc4d-trace` makes vc4d write a CSV time series of
damage, flips, display lists, cursor moves and HVS frames to
`/var/log/vc4d-trace.csv`. The columns are described at the top of vc4d's
`src/trace.rs`.

For measuring only, not for cards people use: a sampler thread wakes every
millisecond, every frame and every cursor move writes a line, and the file
grows without limit. vc4d-flip turned it on until 2026-11-20; since then it is
off unless this experiment is in the build.

```
scripts/build-image.sh --experiment vc4d --experiment vc4d-flip --experiment vc4d-trace ...
```

`desktop-check.py --trace` needs it.
