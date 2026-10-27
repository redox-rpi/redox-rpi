# Open tasks

Small, self-contained tasks for contributors and their AI agents. Before you
start, read [CONTRIBUTING.md](../CONTRIBUTING.md) and claim the task in its
issue. "Needs" says which hardware the task requires: **QEMU** tasks run on
any Linux build host, **Pi** tasks need a Raspberry Pi 3B+, usually with a
USB-serial adapter on its console.

## Stability

### T1: Measure the remaining start crashes in QEMU (QEMU)

Dynamically linked programs still fail at start: about 2 of 100 starts in
QEMU, 36 of 300 on a real Pi with relibc patch `0005` (`ld.so` reads a
malformed ELF file, or `brk #1` with "no filetable fd present"). With the
`ldso-diagnostics` experiment, which reads every byte instead of one per
page, the Pi had 0 of 300 ([test reports](test-reports.md)).

- **Do:** Build the default netboot image and run the start loop from
  [debugging-random-crashes.md](debugging-random-crashes.md) for at least
  1000 starts. Count failures and collect the crash dumps from
  `images/qrun.log`. Also try an experiment that reads every 64th byte
  (one per cache line) instead of every byte or one per page.
- **Done when:** a table with the rates, the build IDs and the crash
  addresses is in the task's issue.

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

### T4: Signal mask bug in the aarch64 signal entry (Pi)

In relibc's `redox-rt/src/arch/aarch64.rs`, the code at label `3` in
`__relibc_internal_sigentry` stores the signal number where the signal
word belongs, so after one `pthread_kill` all signals of that thread stay
blocked. Shown in QEMU by `sigregs thread`
([test reports](test-reports.md)); experiment `sigentry-thread-signals`
fixes it and passes in QEMU and on a real Pi.

- **Do:** review the patch against the x86_64 version of the same
  function (x86_64 also adds 64 to the signal number of signals sent to a
  thread; aarch64 does not) and decide whether it becomes relibc patch
  `0007`.
- **Done when:** reviewed, and either moved to `patches/relibc/` or the
  reason it stays an experiment is in its README.

### T5: Programs exit with code 1 and print nothing (QEMU or Pi)

`ps` straight to the terminal often exits with code 1 and no output;
`ps | cat` and `ps > file` work. `reboot` showed the same: exit code 1, no
message, it took several attempts. Seen over devshd on the Pi.

- **Look at:** `ptyd` in base, how uutils `ps` handles a failed write, and
  whether these are start failures that cannot print their message.
- **Done when:** the cause is known and either fixed as a patch or
  documented in [known-issues.md](known-issues.md).

### T11: devshd sessions leave `login` processes behind (Pi or QEMU)

After about 30 devshd sessions, 31 `login` processes were still running.

- **Look at:** `tools/devshd` (how it ends a session and waits for its
  child) and `login` in userutils.
- **Done when:** `ps | cat` shows no extra `login` after 20 sessions with
  `scripts/devsh.py`.

### T12: Programs fail at start with "no filetable fd present" (QEMU, Pi)

A dynamically linked `sigregs` dies before `main` in about half of the
starts in QEMU and in every start on a real Pi: `relibc_start_v1` stops at
`.expect("no filetable fd present")` in `src/start.rs` (`brk #1`,
exception class `0x3c`, nothing printed, exit code 1). The thread fd from
the same auxv is there, the filetable fd is not. `ld.so` found it just
before (`src/ld_so/start.rs` has the same check). Likely the same failure
as T5 (`reboot` and `ps` exit with 1 and print nothing) and the `brk #1`
crashes in T1. Linked statically, `sigregs` starts every time.

- **Do:** reproduce with the dynamically linked `sigregs` (build it with
  `template = "cargo"`, see `scripts/build-image.sh`), find out whether the
  auxv on the new stack is incomplete when `ld.so` hands over to the
  program, or whether `relibc_start_v1` reads it from the wrong place.
- **Done when:** the cause is known, with an experiment patch that lets the
  dynamically linked `sigregs` start 100 times out of 100 on the Pi.

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

## Boot

### T15: Boot the kernel straight from the Pi firmware (Pi with serial console)

Today the Pi firmware starts U-Boot, U-Boot provides UEFI for the Redox
bootloader, and the bootloader loads the kernel and initfs from RedoxFS.
Raspberry Pi OS skips all of that: the firmware loads the kernel itself
(`kernel8.img`) and the initramfs (`initramfs` in `config.txt`) and passes
the devicetree, which also says where the initramfs is.

- **Do:** an experiment with a small entry stub in the kernel that does what
  the bootloader does today: build the memory map from the devicetree, find
  the initfs through `/chosen` (`linux,initrd-start`/`-end`), set up the
  first page tables if the bootloader does that on aarch64 (check), and call
  the normal kernel start. Plus a `make-sdcard.sh` mode that puts kernel and
  initfs on the FAT partition.
- **Keep:** U-Boot stays for network boot during development.
- **Open questions:** kernel updates then mean writing to the FAT
  partition; how the image updates it.
- **Done when:** a Pi 3B+ boots from the SD card without U-Boot and the
  Redox bootloader to the login prompt, and `scripts/pi-test.py` passes as
  with the U-Boot image.

## Tooling

### T9: CI builds with the pinned revisions (no hardware)

`.github/workflows/qemu-boot.yml` builds the latest upstream kernel, not the
commit from [config/upstream.lock](../config/upstream.lock).

- **Do:** use the pinned kernel commit; add a weekly job that tries the
  latest upstream and reports whether our patches still apply.
- **Done when:** both jobs run green on a pull request.

### T13: relibc tests on Redox in CI (no hardware, long builds)

[`relibc.yml`](../.github/workflows/relibc.yml) builds relibc and runs
`sigregs` on Linux, but nothing on Redox. The image build takes too long
for a hosted runner.

- **Do:** a job (self-hosted runner on a build host, or a cached
  toolchain) that builds the netboot image with `--experiment
  sigregs-test` and runs `sigregs` in QEMU with
  `scripts/qemu-raspi3b.sh --run`; then relibc's own test suite
  (`tests/`, `make test` with the Redox toolchain) the same way.
- **Done when:** a pull request that brings back the x1/x2 swap
  (experiment `sigentry-x1x2-revert`) turns the job red.

### T14: Deeper analysis of relibc (no hardware)

relibc is the most critical component: every program uses it, and two
bugs in its aarch64 signal entry went unnoticed (see
[test reports](test-reports.md)). Its aarch64 tests are not run upstream.

- **Do:** one at a time, each as its own pull request:
  - unit tests on the host for code that does not need Redox (string,
    number formatting, `ld.so` ELF parsing), with Miri where it runs;
  - a check in CI that the offsets in the aarch64 signal entry match
    `ArchIntRegs` and `SigStack` (disassemble and compare, as in the
    `sigentry-by-name` README);
  - coverage of the tests in `tests/` on Redox, once T13 runs.
- Work in [redox-rpi/relibc](https://github.com/redox-rpi/relibc)
  (CONTRIBUTING.md, step 4); fixes go back as patches here.
- **Done when:** each part runs in CI.

### T10: Start loop in `pi-test.py` with crash details (Pi or QEMU)

`scripts/pi-test.py` counts failed starts, but does not record which
program failed with which message.

- **Done when:** the report lists each failure with program and message.
