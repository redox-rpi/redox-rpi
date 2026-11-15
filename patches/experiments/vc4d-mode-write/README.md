# Experiment: vc4d-mode-write, vc4d programs pixel valve 2

Test only ([#188](https://github.com/redox-rpi/redox-rpi/issues/188),
stage 3a). Build it after `vc4d`, instead of `vc4d-mode`, `vc4d-edid` or
`vc4d-diag` (each of them replaces the vc4d recipe). It builds vc4d from
branch `mode-write` (redox-rpi/vc4d `9b9e176`, on `mode-regs`) and starts
it with `VC4D_MODE=write`.

vc4d reads the EDID and compares pixel valve 2 and the HDMI timing with
the firmware's, as with `vc4d-mode`. When pixel valve 2 agrees, it
programs the valve itself, in the order of Linux's vc4 driver, with the
same values, and reads them back:

```
vc4d: mode: write: programming pixel valve 2 (screen blank for a moment)
vc4d: mode: write: pv2 HORZA     wrote 0x0094002c reads 0x0094002c
...
vc4d: mode: write: PASS: pixel valve 2 reads back as written
```

vc4d's usual checks follow: the HVS channel must still run, then
`vc4d: check: PASSED`. The mode stays the firmware's, so the monitor
keeps it; the screen is blank for a few frames.

Build: `scripts/build-image.sh --experiment timesync --experiment vc4d
--experiment vc4d-mode-write`. Clean `recipes/other/vc4d` first when
another vc4d commit was built before.
