# Code quality baseline (2026-11-08)

First static analysis of our own code, report-only. The targets and the
CI proposal are in the [QMP](qmp.md#code-quality). Run by the Quality
agent on `main` at 3e3e2b39 (plus the quality branch) on the Mac, tools in
a throwaway environment: clippy 0.1.99, shellcheck (shellcheck-py),
ruff and pyflakes, cppcheck 2.22, cargo-audit. Nothing was changed in the
code.

## Summary

- **Our host code is clean.** Python: no pyflakes finding. Shell: every
  shellcheck error and warning was checked by hand and is a false
  positive. Tool crates: 12 clippy warnings, all style. No advisories in
  any `Cargo.lock`.
- **The risk is in code no analysis reached** (drivers: since 2026-11-11 `scripts/clippy-redox.py`, see below). The drivers we add as base
  patches (dwc2d 5 076 added lines, pwmaudiod, sdhcid, sdhostd,
  rpi-firmwared, gpio/i2c/spi) and our kernel and relibc patches run
  clippy nowhere: they build only for the Redox target inside the
  cookbook. They hold most of our `unsafe`.
- **vc4d's tests never run.** It has 6 unit tests, but the crate does not
  build on a host (nightly features, `redox-ioctl`, libredox), so
  `cargo test` fails on both the Mac and the build host.
- **Few host tests.** 6 of 20 tool crates have tests (30 tests, all pass);
  line coverage is not measured anywhere.
- **Undocumented `unsafe`.** 51 `unsafe` blocks in 11 tool crates have no
  `SAFETY:` comment (icachetest 21 and sigregs 13 are test tools by
  nature).

## Rust: clippy (default lints)

`cargo clippy` per crate in `tools/`.

| Crate | Warnings | Note |
|---|---|---|
| cputop | 2 | |
| redoxfs-file | 2 | |
| socdump | 2 | |
| timerd | 3 | casts to the same type, `map_or` |
| timesync | 1 | |
| tone | 1 | |
| redox-fatfs2 | – | does not build on macOS (libredox needs Linux or Redox) |
| rpi-boot-stub | – | needs `REDOX_KERNEL` and a bare-metal target |
| the other 12 | 0 | |

relibc: `relibc.yml` already runs clippy report-only on the mirror.

## Rust: clippy for the Redox target (2026-11-11)

`scripts/clippy-redox.py` (#272) on our drivers in base, `main` with
`patches/base` applied, `cargo clippy --target aarch64-unknown-redox
-Zbuild-std` with `clippy::undocumented_unsafe_blocks` (only type-checks,
22 s on homenodellm). Each crate's own warnings only.

| Crate | Warnings | of them undocumented `unsafe` |
|---|---|---|
| bcm2835-sdhcid (upstream driver, patched) | 66 | 14 |
| bcm2835-pwmaudiod | 17 | 16 |
| dwc2d | 16 | 3 |
| bcm2835-sdhostd | 9 | 8 |
| rpi-firmwared | 5 | 4 |
| bcm2835-gpiod, -i2cd, -spid | 3 each | 3 each |
| **total** | **122** | **54** |

Not yet: kernel patches (needs its own target spec), relibc patches
(`relibc.yml` runs clippy on the mirror) and vc4d.

## Rust: `unsafe`

`unsafe` blocks, functions and impls. Tools: whole files; patches: added
lines (`+`) of `.rs` files; vc4d: branch `scanout-diag`.

| Where | `unsafe` | Rust lines |
|---|---|---|
| tools/rpi-boot-stub | 32 | 880 |
| kernel patches (default) | 25 | 565 |
| tools/icachetest | 23 | 240 |
| base: bcm2835-sdhcid | 16 | 219 |
| base: bcm2835-pwmaudiod | 16 | 1 258 |
| vc4d | 17 | 1 564 |
| tools/sigregs | 13 | 339 |
| base: bcm2835-sdhostd | 9 | 972 |
| relibc patches (default) | 6 | 352 |
| experiments (recipes, kernel, relibc) | 16 | 1 140 |
| base: rpi-firmwared | 4 | 976 |
| base: dwc2d | 3 | 5 076 |
| everything else | 35 | 10 000+ |
| **total** (patches and tools) | **198** | **23 013** |

`unsafe` blocks without a `SAFETY:` comment (`clippy::undocumented_unsafe_blocks`,
tools only): icachetest 21, sigregs 13, devshd 3, socdump 3, timerd 3,
timesync 2, welcome 2, mcast 1, polltest 1, reboot 1, tone 1.

## Shell: shellcheck

36 files (`scripts/*.sh` and the check scripts in experiments): 1 error,
8 warnings, 23 notes.

- Error SC2148 `scripts/rpi-firmware.sh`: no shebang. It is sourced, not
  run: false positive (a `# shellcheck shell=bash` line silences it).
- Warnings SC2054 `scripts/qemu-raspi3b.sh` (3): commas in QEMU arguments,
  meant. SC2154 `cpu-ondemand/qemu-check.sh` (3): variables set by
  `printf -v`, shellcheck cannot see them. SC2164 (2): `cd` without
  `|| exit` in `build-visit-images.sh` and `qemu-run.sh`, both under
  `set -e`.
- Notes: mostly SC2086 (unquoted variables, 9; 7 in `test-hotplug.sh`)
  and SC2015 (`a && b || c`, 6).

## Python: pyflakes and ruff

22 files. pyflakes: 0. ruff with E, F, W, B (line length 120): 60, all
style: 31 E741 (variable `l`), 15 long lines, 7 unused loop variables,
2 lambda assignments, 2 imports on one line, 2 statements on one line,
1 `raise` without `from`.

## C: cppcheck

5 files (`tools/tcc-bench`, `tools/chaos`): 1 warning,
`tools/tcc-bench/bench.c:24` uses a `malloc` result without a check. The
rpi-boot-stub is Rust, not C.

## Dependencies: cargo-audit

17 `Cargo.lock` files in `tools/` with 115 packages (mcast, net6 and socdump have none): 0 advisories,
0 warnings (RustSec database of 2026-11-08, 1 422 advisories). Our tools
depend on very little; the large dependency trees are upstream's
(relibc, base, Orbital, COSMIC), outside this scope.

## TODO and FIXME

Our code and the added lines of all patches: 2 comments, neither names an
issue:

- `patches/experiments/ipv6/relibc/0002-…`: `// TODO: safe transmute`
- `patches/experiments/sshd/relibc/0043-…`: `TODO: pass the executable's
  path (AT_EXECFN) instead.`

## Host tests

| Crate | Tests | Result |
|---|---|---|
| assistant | 13 | pass |
| socdump | 8 | pass |
| timerd | 4 | pass |
| timesync | 2 | pass |
| welcome | 2 | pass |
| cputop | 1 | pass |
| vc4d | 6 | do not build on a host |
| the other 14 tool crates | 0 | |

## How to repeat

```bash
python3 -m venv /tmp/qa && /tmp/qa/bin/pip install ruff pyflakes shellcheck-py cppcheck
git ls-files '*.sh' | xargs /tmp/qa/bin/shellcheck -f gcc
git ls-files '*.py' | xargs /tmp/qa/bin/pyflakes
git ls-files '*.c' | xargs /tmp/qa/bin/cppcheck -q --enable=warning,portability,performance
for c in tools/*/Cargo.toml; do cargo clippy --manifest-path "$c" -- -W clippy::undocumented_unsafe_blocks; done
cargo install cargo-audit && for l in tools/*/Cargo.lock; do cargo audit --file "$l"; done
```
