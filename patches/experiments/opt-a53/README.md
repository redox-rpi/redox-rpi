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
in-order core. It also turns on the `aes` and `sha2` target features (see
`rustc --print cfg -C target-cpu=cortex-a53`), but the A53 of the BCM2837
has no ARMv8 crypto extension: code that checks `target_feature = "aes"`
at compile time would use instructions the Pi does not have. So every
`target-cpu=cortex-a53` comes with `-C target-feature=-aes,-sha2`, and the
kernel target spec gets `-aes,-sha2` in its features. `crc` stays on: the
BCM2837 has the CRC32 instructions. (Image B of row 260 was built before
this fix; nothing in it used the AES or SHA2 instructions, it passed all
tests.)

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

### Build and sizes, 2026-11-06

B is `v0.4.0-dev-567-gd91a247f+x9b71` (3 minutes on homenodellm, from the
stable build's cache). The cargo fingerprints of base show
`-Ctarget-cpu=cortex-a53`. Both images are on homenodellm in
`~/redox-rpi-opt/keep/A/` and `keep/B/`.

Every part got smaller, although `opt-level 3` allows larger code: LTO
drops what no program uses.

| File | A (bytes) | B (bytes) | Change |
|---|---|---|---|
| kernel | 1457400 | 1442344 | −1 % |
| initfs (loaded whole by the bootloader) | 25707192 | 20492984 | −20 % |
| `libc.so` | 4357552 | 4002088 | −8 % |
| `netstack` | 1342248 | 1036896 | −23 % |
| `dwc2d` | 750816 | 657168 | −12 % |

### QEMU, 2026-11-06

Function only, no timing. A and B the same: both boot to `redox login:`;
`scripts/pi-test.py` passes version, manifest, ps, ps-tty and 50 of 50
starts; `sdcard` fails on both because QEMU mounts no data partition for
the network-boot image (`/scheme/disk.mmc/0p1: I/O error`).

### Pi, row 260, 2026-11-06

Pass 4, boots in the order B, A, B, A, 3 rounds each: medians of 6 rounds
per image. Raw data on homenodellm in `~/pitest-tools/t260d/opt-a53.jsonl`
(earlier, incomplete passes in `t260`, `t260b`, `t260c`).

| Workload | A (s) | B (s) | B against A |
|---|---|---|---|
| spawn, 500 `ls` | 77.60 | 75.37 | 2.9 % faster |
| net-rx, 32 MiB | 3.02 | 2.59 | 14 % faster |
| net-tx, 32 MiB | 4.89 | 4.38 | 10.5 % faster |
| sd-write, 50 MiB | 6.70 | 6.73 | the same |
| sd-read, 50 MiB | 0.56 | 0.55 | the same |
| boot to `redox login:` | about 30 | about 30 | the same (35–40 s outliers on both) |

`scripts/pi-test.py` passed on all boots of pass 4. In pass 3, one boot of
B lost the network during the start loop (no panic, nothing on serial); it
did not come back in pass 4, where B ran 600 more starts. Neither image
has the futex fix of #197.

The network gains come from `netstack` and `dwc2d`, which today are built
with `opt-level s`. The SD card does not change: its time goes to the card
and the driver waiting for it, not to the CPU.

The bench script found two things about Redox's `nc`: `nc -l` needs a full
address (`0.0.0.0:5135`), and `nc` does not close the connection at the
end of its input.

### Pi, row 289, 2026-11-08: which part gives the gain

Effects measured separately. B2 is B rebuilt with `-aes,-sha2`. C is
experiment [opt-speed](../opt-speed/): opt-level 3, fat LTO and 1 codegen
unit, without target-cpu. D2 is experiment [cpu-a53](../cpu-a53/):
target-cpu cortex-a53 only. 8 boots in the order A, B2, C, D2, D2, C, B2,
A; medians of 6 rounds per image. Report in `docs/test-reports.md` on main
(44c0f9c2), and a [comment on PR 226](https://github.com/redox-rpi/redox-rpi/pull/226#issuecomment-6718340638).

| Image | Build ID |
|---|---|
| A | `v0.4.0-dev-565-gc70f394e+x9f76` |
| B2 | `v0.4.0-dev-575-gd6fb1cf8+x9b71` |
| C | `v0.4.0-dev-574-gf58b489f+xb439` |
| D2 | `v0.4.0-dev-575-gd6fb1cf8+xf367` |

| Workload | A (s) | B2: everything | C: opt-level 3 + LTO | D2: target-cpu only |
|---|---|---|---|---|
| spawn, 500 `ls` | 77.68 | 75.47 (−2.8 %) | 77.09 (−0.8 %) | 79.58 (+2.4 %, slower) |
| net-rx, 32 MiB | 3.07 | 2.69 (−12.3 %) | 2.84 (−7.4 %) | 3.04 (−0.7 %) |
| net-tx, 32 MiB | 4.91 | 4.37 (−11.2 %) | 4.35 (−11.4 %) | 5.00 (+1.7 %) |
| sd-write, sd-read | 6.76, 0.55 | the same | the same | the same |
| boot to `redox login:` | about 30 | the same | the same | the same |

All 8 boots passed `scripts/pi-test.py`.

What it means:

- **opt-level 3 with LTO is the main effect.** Alone, it gives all of
  the net-tx gain and about 60 % of the net-rx gain.
- **target-cpu alone gives nothing**, and program starts get a little
  slower.
- **target-cpu only helps together with LTO.** B2 beats C at spawn
  (−2.8 % against −0.8 %) and at net-rx (−12.3 % against −7.4 %). With
  LTO, LLVM can schedule the whole program for the in-order A53. Without
  it, the scheduling stops at crate boundaries.
- So B2, the full opt-a53 with `-aes,-sha2`, is the best candidate for
  the default build. C is the smaller step if a CPU-specific build is not
  wanted, for example for an image shared with the Pi 4 later.
