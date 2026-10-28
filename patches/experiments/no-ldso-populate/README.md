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

Not run yet.
