# Experiment: opt-speed, compiler settings for speed without the CPU tuning

Half of [opt-a53](../opt-a53/README.md): the same settings without
`target-cpu=cortex-a53`. Kernel: fat LTO, one codegen unit. relibc: one
codegen unit. base: `opt-level 3` instead of `s`, fat LTO, one codegen unit.

Measured against opt-a53 and [cpu-a53](../cpu-a53/README.md) to see which
part of the gain comes from which setting; results in the
[opt-a53 README](../opt-a53/README.md).

Build: `scripts/build-image.sh --experiment opt-speed`.
