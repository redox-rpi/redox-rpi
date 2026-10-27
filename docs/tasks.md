# Open tasks

Small, self-contained tasks for contributors and their AI agents. Before you
start, read [CONTRIBUTING.md](../CONTRIBUTING.md) and claim the task in its
issue. "Needs" says which hardware the task requires: **QEMU** tasks run on
any Linux build host, **Pi** tasks need a Raspberry Pi 3B+, usually with a
USB-serial adapter on its console.

## Stability

### T1: Measure the remaining start crashes in QEMU (QEMU)

Dynamically linked programs still fail at start in about 2 of 100 starts
(`brk #1`, "no filetable fd present").

- **Do:** Build the default netboot image and run the start loop from
  [debugging-random-crashes.md](debugging-random-crashes.md) for at least
  1000 starts. Count failures and collect the crash dumps from
  `images/qrun.log`.
- **Done when:** a table with the rate, the build ID and the crash
  addresses is in the task's issue. Is the faulting address always in the
  same function?

### T2: Find the kernel cause of the start crashes (QEMU)

Relibc patch `0005` (touch fresh memory before copying into it) lowers the
crash rate but does not fix it. The lazy borrow path was ruled out
(experiment `kernel-borrow-populate`).

- **Look at:** page fault handling for anonymous memory in
  `src/context/memory.rs` of the kernel, what happens when a page is
  populated by a write fault from a scheme call in another context.
- **Done when:** an experiment patch that brings the rate to 0 in 1000
  starts without relibc `0005`, with an explanation of why.

### T3: Instruction cache maintenance after loading code (QEMU, then Pi)

Nothing on aarch64 cleans the data cache and invalidates the instruction
cache after `ld.so` or `exec` copied code into memory. QEMU does not need
this; a real Cortex-A53 can execute stale instructions.
`SCTLR_EL1.UCI` is not set, so user space cannot use `dc cvau`/`ic ivau`.

- **Do:** an experiment that does the maintenance, either in the kernel
  (for example when a page becomes executable) or by setting
  `SCTLR_EL1.UCI` and doing it in `ld.so`.
- **Done when:** the experiment builds, the QEMU start loop is not worse,
  and the result on a real Pi (if you have one) is in
  [test-reports.md](test-reports.md).

### T4: Signal mask bug in the aarch64 signal entry (QEMU)

In relibc's `redox-rt/src/arch/aarch64.rs`, the code at label `3` in
`__relibc_internal_sigentry` stores the signal number where the signal mask
belongs. This could leave signals such as SIGCHLD blocked.

- **Do:** compare with the x86_64 version of the same function, fix it as
  relibc patch `0007`, write a small test program that shows the
  difference.
- **Done when:** the patch and the test output before and after.

### T5: `ps` fails when it writes straight to the terminal (QEMU or Pi)

Sometimes `ps` exits with code 1 and prints nothing when its output goes to
the terminal; `ps | cat` and `ps > file` work. Seen over devshd on the Pi.

- **Look at:** `ptyd` in base and how uutils `ps` handles a failed write.
- **Done when:** the cause is known and either fixed as a patch or
  documented in [known-issues.md](known-issues.md).

## Hardware

### T6: sdhost SD driver on the real Pi (Pi with serial console)

Experiment `sdhost-driver` works in QEMU. A netboot image with it did not
come back on the network on a real Pi 3B+.

- **Do:** build with `--experiment sdhost-driver`, boot it on a Pi with the
  serial console attached, find where it stops.
- **Done when:** the boot log is in the issue and, ideally, a fixed patch.

### T7: Power off and reboot (QEMU, then Pi)

`reboot` resets the Pi through the watchdog (kernel patch `0002`).
`shutdown`/power off does not exist yet.

- **Done when:** the Pi stops cleanly, with the power LED state documented.

### T8: Raspberry Pi Zero 2 W board variant (Zero 2 W)

Same SoC as the Pi 3, 512 MB RAM, no onboard Ethernet. See
[ROADMAP.md](../ROADMAP.md), phase for the Zero 2 W.

- **Done when:** the image boots to a login prompt on a Zero 2 W.

## Tooling

### T9: CI builds with the pinned revisions (no hardware)

`.github/workflows/qemu-boot.yml` builds the latest upstream kernel, not the
commit from [config/upstream.lock](../config/upstream.lock).

- **Do:** use the pinned kernel commit; add a weekly job that tries the
  latest upstream and reports whether our patches still apply.
- **Done when:** both jobs run green on a pull request.

### T10: Start loop in `pi-test.py` with crash details (Pi or QEMU)

`scripts/pi-test.py` counts failed starts, but does not record which
program failed with which message.

- **Done when:** the report lists each failure with program and message.
