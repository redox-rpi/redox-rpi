# Rebase on current Redox (study, v0.9)

What it takes to move our patches from the pinned upstream commits
(`config/upstream.lock`) to today's upstream HEAD. Study of 2026-10-30,
patches of main at `cc71069`, by the t18 session. Nothing on main
changes; the refreshed patches and HEAD pins are on branch `rebase-study`
for reference.

## Short answer

The rebase is small right now. Of the 40 default patches, 38 apply to
upstream HEAD as they are. Two need one line each, because upstream added
lines next to ours in a build list. With those two refreshed, the image
**builds and boots in QEMU** at HEAD, with USB network, DHCP and ping, and
with the sdhost experiments. None of our patches is in upstream yet, so
none can be dropped. One branch experiment can (`timer-sigmask`). About
half a day for the defaults, most of it the Pi tests.

## Upstream since our pins

| Source | Pinned | HEAD (date) | Commits | What changed that matters to us |
|---|---|---|---|---|
| kernel | `c951af42` | `7b1f8e01` (2026-10-28) | 7 | futex fixes, a memory-area refactor, x86 timer work |
| relibc | `c3031359` | `02c368ea` (2026-10-29) | 44 | signal and futex EINTR fixes, `dl_iterate_phdr`, a file call refactor, many lint fixes |
| base | `eddcaa21` | `1e4db7a3` (2026-10-29) | 65 | `virtio-fsd`; `udsd` split out of `ipcd` (a new initfs daemon); **libredox 0.2, redox_event 0.5, redox-scheme 0.13 (git), orbclient from git** |
| bootloader | `cc922ec0` | `11b85164` (2026-10-27) | 2 | retry `ExitBootServices` |
| fdt | `8fa4388e` | same | 0 | – |
| redox (build system) | `e079d707` | `70a3f9e1` (2026-10-28) | 25 | toolchain: libunwind instead of compiler-rt, deterministic prefix; package updates |

The library bumps in base looked like the biggest risk, because `dwc2d`,
`bcm2835-sdhcid`, `rpi-firmwared` and `bcm2835-sdhostd` use them. They
compile unchanged.

## Default patches

Checked by applying each patch in order to HEAD with `git apply`
(`--3way` and `--reject` where it did not apply), see "How" below.

| Patches | Result | Work |
|---|---|---|
| kernel `0001`–`0007` | apply as they are | none |
| relibc `0001`–`0007` | apply as they are | none |
| bootloader `0001`, `0002` | apply as they are | none |
| fdt `0001` | applies (fdt did not change) | none |
| base `0001` (dwc2d) | **conflict** in `Cargo.toml`: upstream added `virtio-fsd` and a `selftests` entry next to the workspace members; our `"drivers/usb/dwc2d",` line must go in again | 5 minutes, done on `rebase-study` |
| base `0002`–`0016` | apply as they are | none |
| base `0018` (rpi-firmwared) | **conflict** in `Makefile`: upstream added `SELFTEST_DIRS` and `udsd` lines where our `INITFS_BINS += rpi-firmwared` block sits | 5 minutes, done on `rebase-study` |
| base `0020`–`0024`, `0030` | apply as they are | none |

Already in upstream (patch applies in reverse): none. Our Pi drivers,
the PL011 and BCM2835 interrupt work, the signal entry fixes in relibc and
the ld.so changes are all still ours.

## Experiments on main

All apply to HEAD on top of the defaults: `kernel-borrow-populate`,
`t1-memmap` (kernel); `ldso-diagnostics`, `sigentry-by-name`,
`sigentry-thread-signals`, `sigentry-x1x2-revert` (relibc);
`sdhost-driver`, `sdhost-boot`, `sdhost-stats` (base). The other
experiment directories on main have no source patches (`timesync`,
`cpu-clock`, ...) or are empty since they became default patches.

## Experiments on other branches

Each branch experiment was applied alone on HEAD plus the defaults, and
for comparison on the pinned commits plus the defaults.

**Fail only at HEAD (caused by upstream):**

| Experiment (branch) | Why | Work |
|---|---|---|
| `gpio` 0040 (`bluetooth`), `rpi-fb` 0025 (`fw-direct-boot`), `netstack-multi` 0053 (`netstack-multi`) | `Makefile` build lists, the same kind as base `0018` | 5 minutes each, by the owner |
| `gpio` 0042 | follows from 0040 not applying | none once 0040 is refreshed |
| `sshd` relibc 0001 (`sshd`) | `alarm.rs`: upstream changed a cast in the context lines | 10 minutes |
| `sshd` relibc 0002 | `platform/redox/mod.rs`: upstream changed the `use` lines in the context | 10 minutes |
| `timer-sigmask` relibc 0001 (`t-timer-sigmask`) | **upstream did the same** (31a421f3 blocks all signals in the timer thread) | drop it |

**Fail at the pinned commits too** (they need another experiment first,
e.g. `usb-hotplug` or `gpio`; not a rebase problem, not checked in their
combinations): `bluetooth`, `i2c`, `spi`, `dwc2d-boottime`, `usb-mouse`,
`usb-serial`, `usb-storage`, `usb-storage-more`, `hwinfo`, `grow-root`,
`ipv6` (base and relibc), `sdhost-dma`, `wifi-sdio` 0031–0033,
`rpi-firmware` (`zero2w-netboot`), `t12-kernel-diag`.

**Apply at HEAD:** `boot-time` (base and bootloader), `welcome`,
`keyboard-de`, `usb-hotplug`, `cpu-ondemand`, `lan78xx-multicast`,
`multicast`, `pty-sigint`, `sshd` (base), `ctx-leak`, `poweroff`,
`sigreturn-svc`, `icache-sync`, `relibc-tests`, `sigsegv`,
`t12-auxv-dump`, `tls-variant1`, `no-ldso-populate`.

## Build and boot at HEAD

Separate clone, `config/upstream.lock` set to the HEAD commits above,
base `0001` and `0018` refreshed:

- `scripts/build-image.sh --experiment timesync`: builds
  (`v0.3.0-dev-321-gcc71069+x12cb-dirty`, the manifest records the HEAD
  commits). QEMU `raspi3b --usb-net`: boots to the login prompt, `dwc2d`
  finds the hub and the USB network, DHCP and `ping 10.0.2.2` work, `udsd`
  runs (`uds_stream`, `uds_dgram` in `/scheme`).
- With `sdhost-driver sdhost-boot ldso-diagnostics kernel-borrow-populate`
  as well: builds; `bcm2835-sdhostd` serves the card in QEMU.
- `inputd` logs `invalid path ''` once: not new at HEAD and not from boot.
  It shows with the pinned commits as well, right after `ls /scheme`
  (`ls` opens the `input` scheme root with an empty path). Harmless (#1).
- For comparison, upstream's own daily aarch64 image panics in QEMU
  `raspi3b` (`MissingRequiredProperty("interrupt-cells")` in
  `dtb/irqchip.rs`), which our `fdt` patch 0001 fixes.

## Estimate

| Step | Time |
|---|---|
| Refresh base `0001`, `0018` (done on `rebase-study`), move the pins | 30 minutes |
| Full build, QEMU checks (done once here) | 1 hour |
| Pi: standard checks, `sd-stress.py`, 300 starts, network boot and SD boot | 2 hours with the Pi test agent |
| Re-run the relibc tests (signal and futex changes upstream touch the code our signal patches sit in) | 1–2 hours |
| Branch owners refresh their own experiments (table above) | 5–15 minutes each |

Upstream moves fast in base (65 commits in a few days), so the result
holds for days, not weeks. Rebasing often keeps each step this small. A
weekly `scripts/check-patches.sh --latest` (PR #41) would show when a
patch stops applying.

## How

Scripts in the t18 session's work folder on homenodellm
(`~/t18-work/rebase_study.py`, `branch_exps.sh`):

- clone each upstream (blobless) and check out HEAD;
- for each patch in order: `git apply --check`; if that fails,
  `git apply -R --check` (already upstream); then `--3way`; then
  `--reject` to count the rejected hunks, keeping the parts that apply so
  the next patches can still be checked;
- experiments start from the defaults, with the experiments they need
  first (`sdhost-boot` after `sdhost-driver`, ...).
