# Coding standards and rules (draft)

Owner: Architect. Scope and status are listed in the [PMP](pmp.md). This page
grows one section at a time; sections not written yet are listed at the end.

## Drivers: a status file in the scheme

Every driver offers a read-only status file in its own scheme, so a problem
can be looked at on a running system without a debug build.

- **Name:** `status`, for example `/scheme/disk.mmc/status` or
  `/scheme/usb.dwc2/status`. If the scheme's path space has no room for it,
  use `/scheme/<scheme>/.status` and say so in the driver's README.
- **Content:** plain text, one `name: value` per line, grouped under short
  headings. Include what helps find a fault:
  - the hardware state: important registers (raw hex, with the bit fields
    decoded where it is cheap), clocks and their sources, and the bus or
    link speed;
  - what the driver assumed or chose, next to what the hardware reports
    (for example the SDHCI base clock: the value from the capabilities
    register and the value the driver uses);
  - counters since start: requests, bytes, interrupts, errors by kind,
    timeouts, retries, resets;
  - the current state: queue depth, open handles, the last error with its
    time.
- **Cost:** reading the file may read registers, but must not change the
  hardware state or wait for the device. Counters are plain or atomic
  integers, updated on the normal path; no extra logging.
- **Stable enough for scripts:** new lines may be added, but existing names
  keep their meaning. Units go in the name (`_us`, `_bytes`, `_hz`).
- **Kernel:** the same idea in `/scheme/sys/<name>`, for example
  `/scheme/sys/irqstat` (#564) and `/scheme/sys/uart` (experiment
  `serial-rx2`).

New drivers have the file from the start. Existing drivers get it when
someone works on them for another reason, or when a fault needs it.

## Code rules

These rules apply to our own code: `tools/`, `scripts/`, our patches,
vc4d, rpi-open-firmware's own files and tinycc's VC4 parts. They do not
apply to upstream or vendor code that we only carry (for example the
Broadcom headers in rpi-open-firmware, or `Cargo.lock`). In a patch
against upstream, the rules apply to the lines the patch adds, not to the
upstream file around them (for example the kernel's `memory/mod.rs` of
about 1,500 lines). The KPIs that
measure them are in the QMP and the KPI dashboard (#648), and the checks that run in CI are
listed in #656. Reviewers check the rules marked *(review)*. A PR that
breaks a limit says why in its body.

### Simple and small

- **One concern** per function, per patch and per PR.
- **No code for later:** no unused fields, flags, keys or options for
  features that are not built yet, and no `#[allow(dead_code)]` to keep
  them. Document the planned format in the docs page; add the code when it
  is used. *(review)*
- **Limits** (code lines, without blank and comment lines): 600 per Rust or
  C file, 400 per shell or Python script, 60 per function. Above a limit,
  split the file or function, or say in the PR why not.
- **No copies:** the second copy of a block of 6 or more lines becomes a
  function or a shared module (`scripts/lab-env.sh` for shell, a module
  next to the scripts for Python).
- **Few dependencies:** prefer `std` and crates that the Redox workspace
  already has. A PR that adds a crate or library names it in its body:
  what it does, its size, its licence, and why `std` or an existing crate
  is not enough. New crates must pass `cargo deny` (licence, advisories,
  duplicates).
- **Small binaries:** release builds of our crates use `opt-level = "s"`,
  `lto = true`, `codegen-units = 1`, `panic = "abort"` where the crate
  allows it, and stripped symbols. A PR that grows the minimal card
  (initramfs or root) by more than 100 KiB says why.

### Documented

- **Each file** starts with a short comment: what it is for and what it
  talks to (scheme, device, other daemon, script).
- **Public functions** and every non-obvious item get a doc comment. Don't
  write comments that only repeat the code.
- **`unsafe`:** every `unsafe` block has a `// SAFETY:` line that says why
  it is sound.
- **Requirements:** code that implements an acceptance criterion names it
  where the work is done, e.g. `// REQ-NET-1 criterion 2: one line, then
  quiet retries`, and so does the patch header.
- **Issues:** every bug fix and workaround names its issue at the place in
  the code, in the patch header and in the commit message (`fixes #92`),
  as the [PMP](pmp.md) says.
- **Sources:** register values, constants and algorithms taken from Linux,
  a datasheet or a devicetree name their source.

### Tested

- **Pure logic has unit tests:** parsers, conversions, state machines,
  limits and checks are kept apart from hardware access so a host test can
  run them (`cargo test`, or a host C test as in
  rpi-open-firmware `bootcode/tests/`).
- **Every fix brings a test** that fails without it: a unit test where
  possible, else a QEMU check in the regression set, else a Pi test row.
- **Critical parts** (kernel and relibc patches, drivers, init, pkg and
  pkg-persist, the firmware boot path) need a regression-set check or a Pi
  test for every fix. Coverage of host-testable code is measured and must
  not go down; there is no fixed percentage.
- **Analysis tools:** clippy (default lints with `-D warnings`), rustfmt,
  shellcheck, ruff, `-Wall -Wextra -Werror` and cppcheck for C. Report
  only, for now: clippy pedantic, coverage, and code metrics. Miri runs on
  host-testable crates with `unsafe`, and fuzzing on parsers of outside
  input (config files, network packets, file systems), on a weekly timer.

### Errors and logging

- **No `unwrap()` or `expect()` on outside input** (files, network,
  hardware, user input) in daemons, drivers and tools. Return or print the
  error instead.
- **No `todo!()`, `unimplemented!()` or `panic!()` on a runtime path** in
  kernel or driver code that we add or patch (#644). Return an error, or
  log one line and carry on with a safe default.
- **A daemon that cannot continue** prints one line (what failed and why)
  and exits. Retries are quiet after the first line; no line per attempt.
- **Boot output:** one line per event; no harmless error or warning
  lines. Counters go into the driver's `status` file (above), not into the
  log.

### Patches against upstream

- Small, close to upstream, one concern per patch.
- The header names the issue (and the REQ- criterion where there is one)
  and says why the change is needed on the Pi.
- When upstream fixes the same thing, drop our patch (the drift report,
  #309, shows this). Nothing from here goes upstream as code
  (CONTRIBUTING.md).

### Formatting

Automatic, so reviews do not discuss it: rustfmt for Rust (in patches
against upstream, with upstream's `rustfmt.toml`), `ruff format` for
Python, the file's existing style for C and shell. Lines up to 100 columns
in our own files.

## Still to write

- Commit messages
- Language-specific details beyond the rules above (Rust editions, C
  standard, shell `set -euo pipefail`, Python version)
