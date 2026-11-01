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
| base | 28 (0001–0037) | 26 apply as they are; **0001** (dwc2d) and **0018** (rpi-firmwared) need a 3-way merge (refreshed since, see below) | all apply |
| bootloader | 2 | apply as they are | — |
| fdt | 1 | applies as it is | — |

The two 3-way merges are context only: upstream added lines to `Cargo.lock`,
to the workspace `members` in `Cargo.toml`, and before the aarch64 block of
the `Makefile` (`virtio-fsd`, `udsd`). Our added lines stay the same.
Refreshing the base patches with `git format-patch` gives the same patches
with new line numbers.

**Refreshed on 2026-10-31** (branch `refresh-0001`): `0001` and `0018` now
apply as they are to both the pinned base commit and the head (`2ec1b19d`),
so `scripts/check-patches.sh --latest` gets past them. Their hunks for
`Cargo.toml` and the `Makefile` keep only context that is the same in both:
`dwc2d` now comes after `xhcid` in the workspace `members` (order does not
matter there), and `0018`'s Makefile hunk has one line of leading context.
On the pinned commit the patched tree differs from before only in that
order.

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

## Update 2026-11-01: fdt 0.2 in base, storage patches

Checked by the SD Card & Storage session against base `8424d392`
(2026-10-31) and redoxfs master `5ba45a6`, each patch in order with a 3-way
merge (the one-off check, as above).

- **base moved to fdt `0.2.0-alpha3`** (`919f8268` "Update to fdt
  0.2.0-alpha3", `Cargo.toml`: `fdt = "0.2.0-alpha3"`). Upstream's own
  drivers changed with it: `reg.starting_address as usize` became
  `reg.address`, and `ranges()` now needs
  `.iter::<usize, usize, usize>()` with `Result` items and `len` instead of
  `size`. Every base patch of ours that reads the devicetree uses the old
  API: `0001` (dwc2d), `0018` (rpi-firmwared), `0025` (bcm2835-sdhostd),
  `0045`/`0047`/`0048` (GPIO, I2C, SPI). **Correction:** they still
  compile against the new base. Each of these drivers names its own
  `fdt = { git = "https://github.com/repnop/fdt.git", rev = "059bb23" }`
  and not the workspace `fdt`, so cargo builds that old fdt as a second
  crate next to 0.2 (checked: unchanged `bcm2835-sdhostd` passes
  `cargo check` on `8424d392` with all default patches up to `0028`). Our
  `fdt` pin (`8fa4388` with `fdt/0001`) is only patched into the kernel
  build, not into base, so it doesn't change this. The port to fdt 0.2 is
  clean-up, not a build fix: one fdt in base, and the same API as
  upstream's drivers. It can wait for the lock move.
- **Pattern for the port**, from upstream's `bcm2835-sdhcid` in `919f8268`
  and our `bcm2835-sdhostd` (experiment `sdhost-fdt02`, branch
  `sdhost-fdt02`):

  | fdt 0.1 (`git 059bb23`) | fdt 0.2 (`fdt.workspace = true`) |
  | --- | --- |
  | `fdt = { git = ..., rev = "059bb23" }` in `Cargo.toml` | `fdt.workspace = true` |
  | `Fdt::new(&dtb)` | `Fdt::new_unaligned(&dtb)` |
  | `use fdt::Fdt;` and `fn f(fdt: &Fdt)` | a type alias (below), `Fdt` is generic over its parser |
  | `fdt.find_compatible(&with)` | `fdt.all_compatible(&with).next()` |
  | `node.reg()?.next()` | `node.reg()?.iter::<usize, usize>().next()?` gives a `Result` |
  | `reg.starting_address as usize` | `reg.address` |
  | `reg.size.unwrap()` | `reg.len` |
  | `ranges.find(\|r\| ... r.size)` | `ranges.iter::<usize, usize, usize>()` with `Result` items, `r.len` |
  | `node.property("x")` (raw bytes) | `node.raw_property("x")` |

  `sdhost_phys` before and after:

  ```rust
  // fdt 0.1
  let bus_addr = fdt
      .find_compatible(&["brcm,bcm2835-sdhost"])
      .and_then(|node| node.reg())
      .and_then(|mut reg| reg.next())
      .map(|reg| reg.starting_address as usize)
      .unwrap_or(SDHOST_BUS_ADDR);
  fdt.find_node("/soc")
      .and_then(|soc| soc.ranges())
      .and_then(|mut ranges| {
          ranges.find(|r| {
              r.child_bus_address <= bus_addr && bus_addr - r.child_bus_address < r.size
          })
      })
      .map(|r| r.parent_bus_address + (bus_addr - r.child_bus_address))
      .unwrap_or(bus_addr)

  // fdt 0.2
  type Fdt<'a> = fdt::Fdt<
      'a,
      (fdt::parsing::unaligned::UnalignedParser<'a>, fdt::parsing::Panic),
  >;

  let bus_addr = fdt
      .all_compatible(&["brcm,bcm2835-sdhost"])
      .next()
      .and_then(|node| node.reg())
      .and_then(|reg| reg.iter::<usize, usize>().next())
      .and_then(Result::ok)
      .map(|reg| reg.address)
      .unwrap_or(SDHOST_BUS_ADDR);
  fdt.find_node("/soc")
      .and_then(|soc| soc.ranges())
      .and_then(|ranges| {
          ranges
              .iter::<usize, usize, usize>()
              .filter_map(Result::ok)
              .find(|r| r.child_bus_address <= bus_addr && bus_addr - r.child_bus_address < r.len)
      })
      .map(|r| r.parent_bus_address + (bus_addr - r.child_bus_address))
      .unwrap_or(bus_addr)
  ```

  The type alias is only needed where an `Fdt` is passed to a function;
  code that keeps it in a local `let` (like `sdhcid`) needs none.
  Interrupts, phandles and `#address-cells` changed more: see upstream's
  `drivers/pcid/src/cfg_access/mod.rs` in `919f8268`
  (`dt.root().resolve_phandle(PHandle::new(..))`,
  `node.property::<AddressCells>()`, `node.property::<InterruptCells>()`).
  Quick check without an image build: in a base clone at the new rev with
  our patches applied, `cargo check -p <driver> --target
  aarch64-unknown-redox` with the cookbook's
  `prefix/aarch64-unknown-redox/rust-install/bin` first in `PATH`.
- 3-way results on base `8424d392`, all default patches in order: **`0015`**
  (sdhcid pin routing) conflicted on the ioremap `println` that upstream
  changed. Fixed on branch `storage-upstream`: the call moved into the init
  block, and it now applies to both the pinned and the newest base. Still
  conflicting: `0024` (getty) and `0045`, `0047`, `0048` (GPIO, I2C, SPI,
  workspace lines). `0001` and `0018` apply with 3-way (PR #124 makes them
  apply as they are). The sdhost patches `0025`–`0028` apply.
- **redoxfs master** (`5ba45a6`, 4 commits after our pin `db847d87`) is the
  version that broke Unix sockets (PR #146). `0036`, `0038` and `0039`
  apply; `0037` conflicts on import lines only (the 2024 edition commit).
  Nothing to do until upstream fixes the sockets.


## Update 2026-11-01: network patches

Checked by the network session against base `8424d392` and relibc
`545a55c4` (both 2026-10-31), next to the pinned `eddcaa21` and `c3031359`.
Each patch was applied in order, as it is first and with a 3-way merge only
if needed; then `cargo check --target aarch64-unknown-redox` of `netstack`,
`dhcpd`, `dwc2d` and, where the branch has them, `testnetd` and
`bcm2835-sdiod`, and of relibc for the IPv6 patches.

- **Apply as they are to both bases:** the network default patches on
  `main` (`0005`–`0010` CDC and LAN7800, `0020` RTL8152/8153, `0022`,
  `0023`, `0032` dhcpd, `0056` netcfg `getdents`); `0041`–`0043` and `0058`
  (PRs #81, #118); `0049` (PR #90); `0082` (PR #144); the Wi-Fi patches
  `0030`–`0034` of experiment `wifi-sdio` (branch `wifi-join`); the
  experiments `ipv6`, `multicast` and `lan78xx-multicast` in base (PRs #82,
  #93).
- **Fixed:** `0044` (testnetd, PR #81) needed a 3-way merge on the
  Makefile: upstream added `udsd` to the initfs lines above it. Its hunk now
  keeps only lines that are the same in both (`netstack-multi` `12aa285`,
  merged into `netstack-multi2`). The IPv6 relibc patch `0002` (PR #82)
  needed one for `bind()`, where upstream added a signal lock line; its hunk
  now starts below it (`ipv6` `e75c49c`, merged into `mdns`).
- **Builds:** all of the above pass `cargo check` on both bases, also with
  `0001` and `0018` merged 3-way (PR #124). No change to netstack's or
  smoltcp's API upstream touches us.
- Branch `wifi-join` still carries the old `0044`; it gets the fixed one
  when it takes `netstack-multi` again.


## Update 2026-11-01: graphics patches

Checked by the graphics session against base `8424d392` and kernel
`1a1295b0` (both 2026-10-31), next to the pinned `eddcaa21` and `c951af42`.
The graphics work is all in experiment `rpi-fb` on draft PR #113 (branch
`rpi-fb-pr`): base `0060`–`0072` (`rpi-firmwared` property file,
`rpi-fbd`) and kernel `0049`; experiment `orbital` is a config file only.
Branches `graphics-fb`, `graphics-fb2` and `graphics-fb3` are older subsets
of it, with the old numbers, and are not kept up.

- **Drift was on our side, not upstream's.** PR #113 no longer built on
  current `main` (pinned base): `0060` conflicted with `main`'s own
  `rpi-firmwared` patches `0040` (governor) and `0046` (power LED), which
  landed after the PR branched. `main` names its single-tag helper `call()`,
  so `0060`'s raw message call is `call_raw()` now, and `set_property()`
  keeps the empty answer from `0046`. `0061` also had to add `rpi-fbd` to the
  `INITFS_BINS` line that `0045`/`0047`/`0048` extended. Fixed on
  `rpi-fb-pr` (`123ca91`, `fbe140a`, with `main` merged in), so PR #113
  covers it and no extra PR is needed.
- **Newest base:** with the open PRs' versions of the default patches
  (#161 `0015`, #164 `0024`, #165 `0045`/`0047`/`0048`), all of
  `0060`–`0072` apply. The only fix was `0061`'s `Makefile` hunk: upstream's
  new `SELFTEST` lines sit above the aarch64 block, so it now has one line
  of context, like the `0045`/`0047`/`0048` hunks in #165. The same files
  apply to the pinned base.
- **Kernel `0049`** (13 orders, blocks of up to 16 MiB) applies as it is to
  `1a1295b0` with offsets. Upstream's new code in `src/memory/mod.rs`
  already sizes its own arrays with `ORDER_COUNT`; the closure the patch
  changes still has the hard-coded `11`. `git am` refuses the file only
  because it has no mail header; `patch`, which the build uses, takes it.
- **fdt:** `rpi-fbd` (DMA channel, `dma-ranges`) and `rpi-firmwared` name
  their own `fdt` git rev `059bb23`, like the other drivers in the storage
  section above, so they build next to fdt 0.2. Porting them is the same
  clean-up as there and can wait for the lock move.
- **Builds:** `cargo check -p rpi-fbd -p rpi-firmwared --target
  aarch64-unknown-redox` passes on both bases. Image
  `v0.3.0-dev-908-gfbe140a+x4af4` (pinned, `--experiment timesync
  --experiment rpi-fb`) in QEMU with the Pi's memory layout: CPU blits by
  default, and with `RPI_FB_DMA=on` the DMA test of the whole screen, then
  `fbcond` handoff and login.
- Pi rows #148 (default) and #151 (`RPI_FB_DMA=on`) now name this image.
  The screen checks stay with the screen visit (#34, #43, #44, #54 with
  #108).
