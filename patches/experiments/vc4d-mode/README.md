# Experiment: vc4d-mode, mode registers next to the firmware's

Test only ([#188](https://github.com/redox-rpi/redox-rpi/issues/188),
stage 2c), after `vc4d`, instead of `vc4d-edid` or `vc4d-diag` (each of
them replaces the vc4d recipe). Builds vc4d from branch `mode-regs`
(redox-rpi/vc4d `688038e`, on `edid`).

vc4d reads the EDID as with `vc4d-edid`. Then, once it has found the
firmware's display running, it reads pixel valve 2 and logs, for each
register, what it would write for the EDID's preferred mode next to what
the firmware wrote:

```
vc4d: mode: pv2 HORZA     ours 0x0094002c firmware 0x0094002c
...
vc4d: mode: PASS: pixel valve 2 values for 1920x1080@60 equal the firmware's
```

`DIFFERENT` names the registers that differ. vc4d writes nothing here.
The host tests of the crate `vc4-mode` already show the same values for
the test monitor's EDID (Pi tests 337 and 387).

Build: `scripts/build-image.sh --experiment timesync --experiment vc4d
--experiment vc4d-mode`. Clean `recipes/other/vc4d` first when another
vc4d commit was built before.
