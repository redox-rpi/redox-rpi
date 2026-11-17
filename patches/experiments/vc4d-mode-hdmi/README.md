# Experiment: vc4d-mode-hdmi, vc4d writes pixel valve 2 and the HDMI timing

Test only ([#188](https://github.com/redox-rpi/redox-rpi/issues/188),
stage 3b). Build it after `vc4d`, instead of `vc4d-mode-write`,
`vc4d-mode`, `vc4d-edid` or `vc4d-diag` (each of them replaces the vc4d
recipe). It builds vc4d from branch `mode-hdmi` (redox-rpi/vc4d
`fc40285`, on `mode-write`) and starts it with `VC4D_MODE=write`.

This is `vc4d-mode-write` (stage 3a, Pi test 444) plus one step. When
the HDMI timing compares equal (which needs the HSM clock running), vc4d
also writes the HDMI core's six timing registers. They go in the same
blank moment as pixel valve 2, with the same values as the firmware's,
and vc4d reads them back:

```
vc4d: mode: write: programming pixel valve 2 and the HDMI timing (screen blank for a moment)
vc4d: mode: write: pv2 HORZA     wrote 0x0094002c reads 0x0094002c
...
vc4d: mode: write: hdmi HORZA     wrote 0x00006780 reads 0x00006780
...
vc4d: mode: write: PASS: pixel valve 2 and the HDMI timing read back as written
```

The encoder, its scheduler, the PHY and the clocks stay as the firmware
set them.

Build: `scripts/build-image.sh --experiment timesync --experiment vc4d
--experiment vc4d-mode-hdmi`. Clean `recipes/other/vc4d` first when
another vc4d commit was built before.
