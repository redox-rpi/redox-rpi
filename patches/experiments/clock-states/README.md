# Experiment: clock-states

Issue [#126](https://github.com/redox-rpi/redox-rpi/issues/126), power-saving
mode: which of the firmware's clocks run at idle? A clock that runs for a
block nothing uses is a candidate for gating.

```bash
scripts/build-image.sh --experiment clock-states
```

Base patch `0091` (`rpi-firmwared`): `/scheme/rpi-firmware/clock_states`
has one line per clock the firmware has (`emmc uart arm core v3d h264 isp
sdram pixel pwm`): name, `on` or `off`, the measured rate in Hz
(GET_CLOCK_MEASURED, `0x00030047`) and the firmware's state word
(GET_CLOCK_STATE, `0x00030001`), for example `arm on 1400000000 0x0`. A
clock is `on` when the firmware measures a rate. Clocks the firmware does
not have (state bit 1) are left out. The `clocks` file (rates) is
unchanged.

Pi test 231 showed why the state word is not enough: it was 0 ("off") for
the running ARM, core and SDRAM clocks and 1 only for `uart`. It seems to
say whether a clock was asked for, not whether it runs.

Nothing is switched yet. If a clock of an unused block (for example `v3d`,
`h264`, `isp`, `pwm`) runs at idle, a later step gates it and measures.

QEMU: [`qemu-check.sh`](qemu-check.sh) reads the file. QEMU's firmware
answers every clock tag in its own way, so the states there say nothing
about the Pi.
