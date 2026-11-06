# Experiment: opt-a53, compiler settings for speed

Does Redox on the Pi get faster when kernel, relibc and base (all daemons
and drivers) are built for speed and for the Pi's CPU? Only compiler
settings change, no code.

## What the stable build uses today

| Part | Today | This experiment |
|---|---|---|
| kernel | `opt-level 3`, no LTO (`#lto = true` commented out), 16 codegen units, generic ARMv8 | fat LTO, 1 codegen unit, `"cpu": "cortex-a53"` in the target spec |
| relibc | `opt-level 3`, 16 codegen units, generic ARMv8 (`+v8a`) | 1 codegen unit, `-C target-cpu=cortex-a53` |
| base (daemons, drivers, initfs) | **`opt-level s`** (small code), no LTO, 16 codegen units, generic ARMv8 | `opt-level 3`, fat LTO, 1 codegen unit, `-C target-cpu=cortex-a53` |

The base `Makefile` sets `CARGO_PROFILE_RELEASE_OPT_LEVEL=s` for all daemons
and drivers, so most of the system is built for size, not speed. The
bootstrap stays as it is (fat LTO, `opt-level s`). relibc gets no LTO: it
is a static library combined with C objects, a riskier change for a first
step.

`target-cpu=cortex-a53` lets LLVM order instructions for the A53, an
in-order core. The A53 of the BCM2837 has no ARMv8 crypto extension, so
the setting adds no new instructions, only the scheduling.

## Patches

- [`kernel/0001`](kernel/0001-aarch64-build-the-kernel-for-the-Cortex-A53-with-fat.patch):
  `Cargo.toml` release profile, `targets/aarch64-unknown-kernel.json`
- [`relibc/0001`](relibc/0001-aarch64-build-relibc-for-the-Cortex-A53-with-one-cod.patch):
  `Cargo.toml` release profile, `RUSTFLAGS` in the `Makefile`
- [`base/0001`](base/0001-Build-daemons-and-drivers-with-opt-level-3-fat-LTO-o.patch):
  the two cargo calls in the `Makefile`

Build: `scripts/build-image.sh --experiment opt-a53`.

## Comparison

- **A:** the stable image `v0.4.0-dev-565-gc70f394e+x9f76`
  (`/srv/tftp/backup-v0.4.0-dev-565-gc70f394e+x9f76`).
- **B:** the same commit plus this experiment, nothing else.

On the Pi, for each image: boot time to `redox login:` from the serial log
([`boot-timeline.py`](../boot-time/boot-timeline.py)), then
[`bench.py`](bench.py) from the build host:

```
patches/experiments/opt-a53/bench.py <pi-ip> --label A --rounds 3 --out opt-a53.jsonl
```

It times 500 starts of `ls` (kernel, relibc, ld.so), 32 MiB in each
direction over the network (`netstack`, `dwc2d`), and 50 MiB written to and
read from the SD card (`bcm2835-sdhcid`, RedoxFS). Boot the images in the
order A, B, A, B so a slow boot shows as noise, not as a result. QEMU
timings say nothing here; only the Pi counts.

Also compare the sizes: kernel, initfs (the bootloader loads it whole) and
the image.

## Results

Not measured yet.
