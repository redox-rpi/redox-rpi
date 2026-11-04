# Experiment: clock-states

Issue [#126](https://github.com/redox-rpi/redox-rpi/issues/126), power-saving
mode: which of the firmware's clocks run at idle? A clock that runs for a
block nothing uses is a candidate for gating.

```bash
scripts/build-image.sh --experiment clock-states
```

Base patch `0091` (`rpi-firmwared`): `/scheme/rpi-firmware/clock_states`
lists each clock the firmware has (mailbox tag `0x00030001`,
GET_CLOCK_STATE) as `on` or `off`: `emmc uart arm core v3d h264 isp sdram
pixel pwm`. Clocks the firmware does not have are left out. The `clocks`
file (rates) is unchanged.

Nothing is switched yet. If a clock of an unused block (for example `v3d`,
`h264`, `isp`, `pwm`) runs at idle, a later step gates it and measures.

QEMU: [`qemu-check.sh`](qemu-check.sh) reads the file. QEMU's firmware
answers every clock tag in its own way, so the states there say nothing
about the Pi.
