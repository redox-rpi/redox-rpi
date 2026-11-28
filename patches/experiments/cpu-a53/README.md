# Experiment: cpu-a53, only the CPU tuning

The other half of [opt-a53](../opt-a53/README.md): only
`target-cpu=cortex-a53` (kernel target spec `"cpu": "cortex-a53"`, relibc
and base `-C target-cpu=cortex-a53`). Everything else as today: base stays
at `opt-level s`, no LTO, 16 codegen units.

Measured against opt-a53 and [opt-speed](../opt-speed/README.md); results
in the [opt-a53 README](../opt-a53/README.md).

Build: `scripts/build-image.sh --experiment cpu-a53`.
