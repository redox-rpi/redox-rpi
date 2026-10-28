# Experiment: no-ldso-populate, relibc without patch 0005

For [#2](https://github.com/redox-rpi/redox-rpi/issues/2): reverts relibc
patch `0005` (ld.so touches fresh memory before copying into it). That
patch lowered the start crash rate before the real causes were known:
[#12](https://github.com/redox-rpi/redox-rpi/issues/12) (kernel patch
`0003`) and [#1](https://github.com/redox-rpi/redox-rpi/issues/1) (relibc
patch `0007`). If the start loop is as clean without it, `0005` only
worked around those bugs, and the kernel's page fault handling is not to
blame.

```bash
scripts/build-image.sh --experiment no-ldso-populate
```

## Result

QEMU, 2026-10-28, `ls` and `pwd` 300 times each, `sha256sum` 100 times
per run, output to `/dev/null`:

| Build | Starts | Failed |
|---|---|---|
| `v0.3.0-dev-60-ge2b987b` (`main`, with `0005` and `0007`) | 700 | 0 |
| `v0.3.0-dev-64-gb173d02+xdef1` (without `0005`), two runs | 1400 | 0 |
| before the fixes: `v0.3.0-dev-12-g61a361f+x951b` (with `0005`, without `0007`) | 700 | 8 |

Without `0005` the start loop is as clean as with it. The page population
in `0005` only made the x18 bug (#1) rarer by changing the timing; the
kernel's page fault handling is not to blame. Each run also shows the
known `pcid` exception at boot, which is unrelated.

Dropping `0005` from the default patches would need a test on the real
Pi, where it was introduced.
