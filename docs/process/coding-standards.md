# Coding standards and rules

How code in this project is written, documented and committed. The rules
apply to people and agents alike. Test standards are in
[test-standards.md](test-standards.md), owned by V&V. The requirement
format is in [`docs/requirements/`](../requirements/), owned by
Requirements. The Architect owns this page. Changes to it go through a PR,
and the maintainer merges them.

Each rule has an ID, such as **D1**, that reviews and the quality reports
can cite. **Must** rules are checked, either by a tool or in review.
**Should** rules are the default; a deviation needs a comment that says
why.

## 1. Scope: our code and upstream code

The project has two kinds of code:

| Kind | Where | Which style |
|---|---|---|
| **Our code** | `tools/`, `scripts/`, the files of an experiment that are not patches, the repositories `vc4d` and `rpi-open-firmware` | this page |
| **Upstream code** | patches in `patches/` and `patches/experiments/*/<component>/` against kernel, base, relibc, bootloader, redoxfs, fdt, and the `redox-rpi/relibc` branches | the upstream project's style first, then this page |

- **U1 (must).** In upstream code, write like the code around the change:
  the same naming, error type, logging macros, formatting and comment
  style. If upstream style and this page disagree, upstream wins. The goal
  is a small patch that applies to newer upstream commits
  ([CONTRIBUTING.md](../../CONTRIBUTING.md#rules)).
- **U2 (must).** Never reformat upstream code. Do not run `rustfmt` or
  `clang-format` over a whole upstream file. Format only the lines you
  change. If the upstream file is already `rustfmt`-clean, run `rustfmt`
  and check that the diff touches only your lines.
- **U3 (must).** These rules still apply inside upstream code, because
  they are about our changes:
  - issue references (section 2),
  - SAFETY comments on any `unsafe` we add (R4),
  - the aarch64 rules (section 5),
  - the patch format (section 3).

### Existing code: no mass cleanup

- **B1 (must).** A PR must not add a finding that the Quality baseline
  counts: clippy and ruff warnings, shellcheck findings, cppcheck errors,
  `unsafe` without SAFETY, TODO without issue. A later baseline may count
  more. Existing findings are fixed when someone works on that code anyway,
  or in a cleanup PR of their own. A cleanup PR never mixes with a feature
  or a fix. The counts may only go down.

## 2. Documentation in code

The goal: from any fix or workaround in the code, a reader can reach the
bug, and from the bug back to the code.

- **D1 (must). Every bug fix and workaround names its issue at the place in
  the code.** Use one of these forms, in the comment syntax of the language:

  ```rust
  // Fix for #92: the lease timer restarted on every DHCP offer, so the
  // address expired while the link was up.
  // Workaround for #217: the LAN7800 drops the multicast filter after a
  // reset; write it again. Remove when the driver keeps it.
  // Upstream bug: relibc#309 (x1/x2 swap in the signal entry).
  ```

  - **Fix for #N:** the real fix.
  - **Workaround for #N:** goes around a cause that is still there. Add a
    "Remove when …" sentence.
  - **Upstream bug:** points to a report in the upstream tracker.

  Then one or two sentences on *why*, not *what*. The code shows what.

- **D2 (must). In patches, write `redox-rpi#N`.** In upstream code, a bare
  `#92` would read as the upstream project's issue 92. In our own code a
  bare `#N` is fine. For upstream reports use the project name, such as
  `relibc#309`, or the full URL.

- **D3 (must). TODO, FIXME and XXX only with an issue number:**
  `// TODO(#123): handle the second SD controller.` Without an issue, open
  one first, or leave the comment out. Checked with the regex
  `\b(TODO|FIXME|XXX)\b(?!\(#\d+\))`. In patches, write `TODO(redox-rpi#123)`.

- **D4 (must). The same issue goes into the patch header and the commit
  message**, as a trailer line `Fixes #92` or `Refs #92` (section 4). The
  code comment, the commit and the issue then all point to each other.

- **D5 (should). Comments say why, not what, and carry no history.** "Tried
  X, it did not help" goes into the issue, the experiment's README or the
  topic page, not into the code. A comment that is wrong after a change is
  worse than none: update or delete it in the same commit.

- **D6 (should). Hardware code names its source.** A register constant or
  a timing value names the datasheet and section:

  ```rust
  // BCM2835 ARM Peripherals, 9.6 "Control and Status Register".
  ```

  For the firmware mailbox, link to the firmware wiki property, or say
  that the value was measured on the Pi, with the test number.

- **D7 (should). Public items in our crates have doc comments** (`///`).
  Each crate or program starts with a module comment (`//!`, or a
  docstring in Python, or the header comment in shell). It says what the
  program does and how to call it. This is the pattern `scripts/pi-test.py`
  and `scripts/qemu-raspi3b.sh` already follow.

## 3. Patch format

- **P1 (must).** Patches are made with `git format-patch` from a clone at
  the commit in [`config/upstream.lock`](../../config/upstream.lock), with
  our patches applied (`git am`). Never edit a `.patch` file by hand,
  except to renumber it. CI (`check-patches.sh`) checks that every patch
  applies to the pinned commit.
- **P2 (must). One concern per patch.** A fix, a feature, or a build flag.
  Two independent fixes in one file are two patches. Formatting or
  renaming never travels with a fix.
- **P3 (should). Small and close to upstream.** Change as few lines as the
  fix needs. Do not move code, change public interfaces or add
  dependencies unless the fix needs it. A patch over about 200 changed
  lines should be split, or explained in its header.
- **P4 (must). The header** is the commit message (section 4). Its body
  says what was wrong and why the change fixes it, and ends with
  `Fixes redox-rpi#N` or `Refs redox-rpi#N`.
- **P5 (must).** Numbers in the default directories are given only at the
  merge to `main`. New patches start as an experiment
  ([patches/README.md](../../patches/README.md#patch-numbers)).
- **P6 (must).** When a fix is reported upstream, add its line to the
  "Reported upstream" table in [patches/README.md](../../patches/README.md).
  Once the pinned commit has the fix, drop the patch. Upstream reports are
  the maintainer's: a session drafts them, it does not post them.

## 4. Commit messages

Example:

```
netstack: keep the lease timer across DHCP offers

The timer restarted on every offer, so with two DHCP servers the
address expired while the link was up. Start it once per lease.
Ruled out: the LAN7800 link state (stays up in the serial log).

Fixes #92
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

- **C1 (must). Subject:** `<area>: <what changed>`, at most 72
  characters, no full stop. The area is a component or topic: `netstack`,
  `relibc 0011`, `pi-test`, `docs`. It matches today's history, for
  example `base 0083: refresh against 0041, applies without fuzz again`.
- **C2 (must). Body:** why the change is needed, what it does where that
  is not obvious, and what was ruled out. Short plain English, wrapped at
  about 72 characters.
- **C3 (must). Issue trailer:**
  - `Fixes #N` when the commit closes the issue.
  - `Refs #N` when it only belongs to it.
  - In a patch header, `Fixes redox-rpi#N` (D2).
  - A commit without an issue is allowed only for docs, typos and test-plan
    rows.
- **C4 (must). Agents add their trailer:** `Co-Authored-By: <model>
  <noreply@anthropic.com>`. The PR body names the agent and model and what
  a person did ([CONTRIBUTING.md](../../CONTRIBUTING.md#workflow)).
- **C5 (should). One concern per commit.** No "WIP" or "fix typo" commits
  in a PR that is ready for review. Squash them, or let the squash merge
  do it.

## 5. aarch64 and the Pi 3 CPU

These rules come from bugs that cost days. Each one names the issue that
explains it.

- **A1 (must). Code that saves and restores user state keeps every
  register, x18 included.** This covers the signal entry and return, the
  context switch, fork and `sigreturn`. Redox does not reserve x18: Rust
  and C code from upstream and from the package repository may keep values
  in it. Our signal return used to overwrite it ([#127](https://github.com/redox-rpi/redox-rpi/issues/127),
  [#1](https://github.com/redox-rpi/redox-rpi/issues/1); fixed by
  `relibc/0013` and `kernel/0009`). Every change to such code is tested
  with `sigregs process --x18` and `sigregs thread --x18` on the Pi.
- **A2 (must). Hand-written assembly never uses x18 as a scratch
  register.** This applies to `asm!`, `global_asm!`, `.S` files and code
  generators such as tcc's backend.
- **A3 (must). Keep x18 reserved where it is reserved today:** relibc is
  built with `-Zfixed-x18` and `-ffixed-x18` (`relibc/0007`); tcc and
  `libtcc1.a` with `-ffixed-x18`. A new C toolchain or runtime library for
  the image does the same. Its recipe says why in a comment, as the tcc
  recipe does.
- **A4 (must). No floating point or NEON in the kernel.** The kernel target
  is soft-float with `-neon,-fp-armv8`, and the kernel saves V registers
  only at a context switch.
- **A5 (must). `target-cpu=cortex-a53` always comes with
  `-C target-feature=-aes,-sha2`.** The BCM2837's A53 has no crypto
  extension, but the CPU name turns it on (opt-a53, PR #226). `crc` stays
  on.
- **A6 (should). Addresses say which address space they are in.** ARM
  physical addresses (peripherals at `0x3F00_0000`) and VideoCore bus
  addresses (`0x7E00_0000`, DMA and firmware) are easy to mix up. Constants
  end in `_BUS` or `_PHYS` when it is not clear from the context.
- **A7 (must). MMIO goes through `read_volatile` and `write_volatile`**,
  or through the crate's existing register wrapper. Never use plain
  pointer dereferences. A DMA buffer gets cache maintenance, or uncached
  memory, at the place where it is handed to the device, with a comment
  saying which.

## 6. Rust

- **R1 (must). Formatting:** `cargo fmt` with the default settings in our
  crates. U2 applies in upstream crates.
- **R2 (must). Lints:** `cargo clippy` must not add warnings (B1). Turn off
  a lint only for one item, with a reason:
  `#[allow(clippy::too_many_arguments)] // mirrors the mailbox layout`.
  Never turn one off for a whole crate.
- **R3 (must). No panics on input.** Daemons, drivers and the kernel do not
  `unwrap()` or `expect()` on anything that comes from outside:
  - files and the network,
  - scheme requests,
  - hardware registers,
  - the firmware,
  - user arguments.

  Return an error, log it and go on, or exit with an error code. A daemon
  that panics takes its device or scheme with it, and a kernel panic takes
  the system. `expect("why")` is fine for invariants at start-up that
  cannot fail on a working system. The message says what was expected.
- **R4 (must). Every `unsafe` block, `unsafe fn` and `unsafe impl` has a
  `// SAFETY:` comment** (`/// # Safety` on an `unsafe fn`). It says which
  invariant makes the code sound: who owns the memory, why the pointer is
  valid and aligned, why nothing else touches the register at the same
  time. Keep each `unsafe` block as small as possible. Checked by
  `clippy::undocumented_unsafe_blocks`, which new crates turn on as
  `warn`. Today 76 `unsafe` blocks in `tools/` and 92 added by patches
  have none. B1 applies to them.
- **R5 (should).** New crates set `#![deny(unsafe_op_in_unsafe_fn)]`.
- **R6 (must). Logging:**
  - Daemons and drivers in base use the `log` macros that base sets up
    (`info!`, `warn!`, `error!`, as in `base/0041`).
  - Command-line tools print errors to stderr, prefixed with the program
    name, and exit non-zero.
  - Boot output is short: one `info!` line per daemon that started, more
    only at `debug`. Boot time is measured and counted.
  - Nothing at `info` or above on a hot path, such as per packet, per
    block or per interrupt.
  - Never log secrets: Wi-Fi keys, passwords, tokens.
- **R7 (must). Dependencies:**
  - Only with an MIT, Apache-2.0, BSD, ISC, MPL-2.0 or similar license.
    Never GPL code.
  - The `Cargo.lock` is committed.
  - Each new dependency is named in the PR with what it costs: binary size
    and build time. The image is small, and the Pi is slow.

## 7. C

Our own C: `tools/chaos`, `tools/tcc-bench`, and C in experiments.

- **K1 (must).** C11. Build with `-Wall -Wextra` and add no warnings.
  `cppcheck` must not add findings (B1).
- **K2 (must).** Check every return value of a system call and of
  `malloc`. On error, print to stderr and exit non-zero, or handle it.
- **K3 (should).** Four spaces, K&R braces, `snake_case`. No VLAs. Use
  fixed-width types (`uint32_t`) for hardware and protocol data.
- **K4 (must).** C built for the image with gcc follows A3 where it ends
  up in a runtime library. For programs, pass `-ffixed-x18` as tcc does,
  until the maintainer decides otherwise.

## 8. Shell

- **S1 (must).** Start with `#!/usr/bin/env bash` and `set -euo pipefail`.
  16 of the 23 scripts in `scripts/` do this today. The other 7 get it
  when they are next changed, or keep `set -e` with a comment saying why.
- **S2 (must).** `shellcheck` must not add findings (B1). Turn a finding
  off only on its line, with a reason:
  `# shellcheck disable=SC2086 # word splitting wanted: flag list`.
- **S3 (must).** Quote every variable expansion unless the script splits
  it on purpose (S2). Use `"$@"`, never `$*`.
- **S4 (must).** A header comment with what the script does, its usage and
  its options, as in `scripts/qemu-raspi3b.sh` and `scripts/build-image.sh`.
- **S5 (must).** Scripts that use `build/` or `upstream/` go through the
  build lock (`build-image.sh`, `scripts/with-build-lock.sh`). Never `flock`
  the lock file directly.
- **S6 (must).** No secrets on a command line or in a log: read them from
  their file, such as `~/.config/redox-rpi/`, inside the script.
- **S7 (should).** A script that says it runs on macOS and Linux avoids
  options that differ between GNU and BSD tools (`sed -i`, `date -d`,
  `stat`), or checks which one it has.
- Ion scripts inside the image follow the same ideas: a header comment,
  and a check of the exit status of each step that matters.

## 9. Python

- **Y1 (must).** Python 3, standard library only, so a script runs on
  homenodellm and the Mac without a venv. `scripts/pi-test.py` and
  `tools/log-triage` already work this way. An exception is named in the
  tool's README.
- **Y2 (must).** `ruff check` with the default rules must not add findings
  (B1). New files are formatted with `ruff format`. Existing files are
  formatted only in a cleanup PR of their own.
- **Y3 (must).** A module docstring with what the script does and its
  usage, as in `scripts/pi-test.py`. Use `argparse` for arguments. Exit
  non-zero on failure. A test script counts a check as failed only when it
  failed, not when the script itself broke: it reports that separately.
- **Y4 (should).** Type hints on functions that other scripts import.
- **Y5 (must).** Scripts that are run with `-I`, or that read downloaded
  data, do not import modules from the data's directory. When a script
  imports another script by path (`importlib`), set
  `PYTHONDONTWRITEBYTECODE=1` or clean up afterwards. A stray
  `__pycache__` in `patches/experiments/` makes `build-image.sh` stop.

## 10. Recipes, TOML and configuration

- **T1 (must).** Every unusual flag in a `recipe.toml`, an image config or
  an experiment's `config.toml` has a comment saying why, with its issue.
  Examples: `-ffixed-x18`, `condition_board_not`, a pinned commit.
- **T2 (must).** Configuration that holds secrets is never committed.
  Images read secrets from files on the build host. Release images never
  contain `devshd`.

## 11. Checks

The Quality agent runs these checks and reports the counts. "Block" means
CI fails. "Warn" means CI stays green and writes a note. "Review" means a
reviewer checks it.

| Rule | Tool | Where | Today |
|---|---|---|---|
| R1, Y2 | `cargo fmt --check`, `ruff format --check` (new files) | our code | review; warn once in CI |
| R2, R4 | `cargo clippy` with `undocumented_unsafe_blocks` | `tools/*` crates | baseline, warn |
| S1–S3 | `shellcheck` | `scripts/*.sh` | baseline, warn |
| Y2 | `ruff check` | `scripts/*.py`, `tools/*/*.py` | baseline, warn |
| K1 | `cppcheck`, `-Wall -Wextra` | `tools/*/*.c` | baseline, warn |
| D3 | regex for TODO, FIXME or XXX without `(#N)` | everything we wrote, and added lines in patches | warn |
| R4 in patches | count added `unsafe` without SAFETY | `patches/**` | warn |
| D1, D4, C3 | `Fixes`/`Refs` trailer in commits and patch headers | PRs | review; warn later |
| P1 | `check-patches.sh` | patches | block (exists) |
| U1, U2, P2, P3, R3, R6, A1–A7 | review | PRs | review |

A warning becomes a block only when its count on `main` is zero, and the
maintainer agrees.

## 12. Deviations

A rule that does not fit a case may be broken with a comment at the place
that says why and names an issue. Example:
`// Deviation from R3 (#145): …`. A rule that does not fit often should
change: open an issue for the Architect.
