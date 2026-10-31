# Rebase on current Redox

Analysis for the v0.9 plan item "rebase on current Redox", done on
2026-10-31 by the build agent. No code on `main` changed. It answers: do our
patches still fit the newest upstream sources, and what does moving
`config/upstream.lock` forward cost?

**Short answer:** little. Our pins are a week old. Every default patch and
every experiment on `main` applies to the upstream heads of 2026-10-30, and
two of them need a 3-way merge for context lines only. None of our patches
is upstream yet. With the newest sources the image builds, and in QEMU it
passes all standard checks (0 of 300 starts failed). Four open pull requests
need a small refresh after the move.

## Upstream since our pins

| Source | Pinned (lock) | Upstream head 2026-10-30 | Commits since pin |
|---|---|---|---|
| redox (build system, recipes) | `e079d707` 2026-10-24 | `9007d0a2` | 31 |
| kernel | `c951af42` 2026-10-25 | `7b1f8e01` | 7 |
| relibc | `c3031359` 2026-10-25 | `ad0c4794` | 46 |
| base (drivers, netstack, init) | `eddcaa21` 2026-10-25 | `2ec1b19d` | 73 |
| bootloader | `cc922ec0` 2026-08-25 | `11b85164` | 2 |
| fdt | `8fa4388e` 2026-09-30 | same | 0 |

Each pin is an ancestor of the upstream head, so this is a fast-forward of
the lock with no rewritten history. What changed upstream, as far as it
touches us:

- **kernel:** x86 timer and interrupt changes, a refactor of memory
  sections (`DiscontiguousMemoryArea`), and two futex fixes ("context-wide
  deadlock", "do not return early without rollback"). The futex fixes may
  matter for our thread tests (relibc suite, #13); nothing aarch64-specific.
- **relibc:** `dl_iterate_phdr`, clippy cleanups (fewer `as` casts), and a
  refactor of `src/platform/redox/mod.rs` (`std_fs_call`, `FileTableVerb`;
  `38862a5e`). The refactor is what breaks two open pull requests (below).
- **base:** a new `virtio-fsd`, new versions of `redox-scheme` and
  `libredox`, the `scheme-utils` crate, and Unix sockets in their own
  daemon `udsd` (in the initfs). Our drivers (`dwc2d`, `bcm2835-sdhostd`,
  `rpi-firmwared`) still build against the new crates without a change.
- **redox build system:** toolchain changes (libunwind instead of
  compiler-rt, a deterministic prefix). A fresh clone fetches the new
  prefix; our test build reused the existing one (see "Not checked").

## Our patches on the upstream heads

Each patch was tried in order on the head with `git apply`, then with a
3-way merge, and checked for whether it is already upstream (the reverse
patch applies). Experiments were tried on top of the default patches, with
the experiments they need first.

| Component | Default patches | Result on the head | Experiments on `main` |
|---|---|---|---|
| kernel | 8 (0001–0008) | all apply as they are | all apply |
| relibc | 7 (0001–0007) | all apply as they are | all apply |
| base | 28 (0001–0037) | 26 apply as they are; **0001** (dwc2d) and **0018** (rpi-firmwared) need a 3-way merge | all apply |
| bootloader | 2 | apply as they are | — |
| fdt | 1 | applies as it is | — |

The two 3-way merges are context only: upstream added lines to `Cargo.lock`,
to the workspace `members` in `Cargo.toml`, and before the aarch64 block of
the `Makefile` (`virtio-fsd`, `udsd`). Our added lines stay the same.
Refreshing the base patches with `git format-patch` gives the same patches
with new line numbers.

**Already upstream:** none. Upstream has nothing for the Raspberry Pi since
our pins (no commit in base or kernel mentions BCM2835, dwc2, sdhost or
PL011).

## Test build with the newest sources

In a separate clone on homenodellm (`~/redox-rpi-rebase`, local branch
`rebase-try`, not pushed): `config/upstream.lock` set to the heads above,
the base patches refreshed, then
`scripts/build-image.sh --experiment timesync`, the set of the releases.

- Build `v0.3.0-dev-601-gc1ea123+x12cb`: kernel, relibc, base, bootloader,
  devshd, timesync and reboot built with no error.
- QEMU raspi3b with `--usb-net --data` (PR #110), `scripts/pi-test.py` with
  the manifest: all six checks pass (version, manifest, ps, ps-tty, sdcard,
  starts), 0 of 300 starts failed, no panic. The boot log has the same
  messages as v0.3.0-rc3, nothing new.

## Open pull requests on the upstream heads

The patch set of every open pull request that changes `patches/` was tried
the same way (the lists of `scripts/pr-order.py`, PR #111). Most apply.
These need work after the move:

| Pull request | Patch | Conflict | Cause upstream | Effort |
|---|---|---|---|---|
| #81 netstack-multi, #118 | base `0044` testnetd | `Makefile` | `virtio-fsd` lines next to ours | small: context only. In #118, `0058` then fails too because it builds on `0044` |
| #98 clock-settime | relibc `0011` | `src/platform/redox/mod.rs` | relibc refactor `38862a5e` | medium: rewrite against the new helpers, one function |
| #78 sshd, #101, #120 | experiment `sshd`, relibc `0040`, `0041` | `src/header/unistd/alarm.rs`, `src/platform/redox/mod.rs` | cast cleanup `4cb274c5`, refactor `38862a5e` | medium: two small functions |

The `ipv6` experiment (#82, #93) fails at our current pins as well: it needs
the `multicast` experiment applied first. That is not caused by upstream.
`scripts/check-patches.sh` should list it under `needs()`.

Five pull requests add `redoxfs` to `config/upstream.lock` (#66, #72, #73,
#75, #106). They conflict with a lock move only on neighbouring lines.

## Suggested order and effort

1. **After the v0.3.0 tag** (the release stays on the rc3 pins): one pull
   request that moves `config/upstream.lock` to the upstream heads and
   refreshes the base patches. That is the `rebase-try` commit: the lock and
   new line numbers in 27 base patches, no code change. About an hour with
   CI.
2. **Pi test** of that build: the standard checks, network, SD card stress,
   and one boot on the Zero 2 W. It is an automated row for the Pi test
   agent, about 30 minutes.
3. **Refresh the four open pull requests** after the merge, each by its
   owner: #81 and #118 (minutes), #98 and the `sshd` branches (an hour or
   two each, plus their Pi tests).
4. **From then on, move the lock every week or two.** The weekly
   "Latest upstream" job (`.github/workflows/upstream-weekly.yml`) shows when
   a patch stops applying. At one week behind, the move is mostly
   mechanical; the relibc refactor shows how quickly a month would add real
   work.

## Not checked

- No real Pi. QEMU does not test the SD card controller, USB timing or the
  network hardware.
- Packages the build took from its cache (coreutils, ion, uutils, …) were
  built against the older relibc. A clean build (CI, or a fresh clone) also
  rebuilds them and fetches the new toolchain prefix.
- Only the default patches and `timesync` were built. The other experiments
  were only checked to apply.
- How to repeat: `scripts/check-patches.sh --latest` applies without a 3-way
  merge, so today it stops at base `0001`, and after that patch it checks
  nothing more. The per-patch check here (each patch, 3-way, reverse) was a
  one-off script and is not in the repository yet.
