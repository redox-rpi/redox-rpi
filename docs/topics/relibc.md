# relibc on aarch64

## State

- [redox-rpi/relibc](https://github.com/redox-rpi/relibc) holds the same
  state as [patches/relibc/](../../patches/relibc/): branch `upstream` is the
  pinned commit, `redox-rpi` adds our patches, `exp/<name>` each experiment.
  The patch files here are what the build uses.
- Patch `0004` fixed the x1/x2 swap in the signal entry; CI runs `sigregs`
  on Linux. Reported upstream as
  [relibc#309](https://gitlab.redox-os.org/redox-os/relibc/-/work_items/309):
  once upstream has the fix and `config/upstream.lock` points past it, drop
  `0004`.

## Next

- [#4](https://github.com/redox-rpi/redox-rpi/issues/4) signal mask bug after `pthread_kill` (experiment
  `sigentry-thread-signals`): reviewed against x86_64 on branch
  `t4-thread-signals`, patch `0002` fixes two more differences; Pi tests
  #37 and #68 pass. PR #64 makes both patches default relibc `0008` and
  `0009`; rebuilt on main 2026-10-31, no longer stacked on #62.
- [#40](https://github.com/redox-rpi/redox-rpi/issues/40) prebuilt programs (bash, vim, …) from the Redox package
  repository: experiment `prebuilt-programs`, with experiment
  `sigreturn-svc` ([#1](https://github.com/redox-rpi/redox-rpi/issues/1)) so the signal return no longer overwrites x18,
  which these programs use. Pi test #45 passed (2026-10-29): nine
  programs, 450 of 450 starts. Merged with PR #61. Branch
  `sigreturn-default` makes `sigreturn-svc` default patches, relibc `0013`
  and kernel `0009` ([#127](https://github.com/redox-rpi/redox-rpi/issues/127)).
- Static programs (`sigregs`) carry the relibc they were linked with;
  `build-image.sh` now relinks `sigregs` on every build (branch
  `t1-sigreturn`), before that a relibc-only change kept the old binary.
- [#13](https://github.com/redox-rpi/redox-rpi/issues/13) relibc tests on Redox: experiment `relibc-tests` (branch
  `t13-relibc-tests`) runs upstream's suite in QEMU. `scripts/relibc-tests.sh`
  builds, runs it and compares with `config/relibc-tests-expected.txt`
  (new failure: exit 1; fixed test reported). 319 tests, 25 known failures
  on main, 16 with `tls-variant1` and `sigentry-thread-signals`. Nightly
  unit for the build host ready (01:30 UTC, issue on a change), not
  installed: needs the branch on main. dash as `/bin/sh` makes
  `stdio/popen`, `stdlib/system` and `unistd/exec` pass (22 known
  failures). Draft PR #62.
- [#14](https://github.com/redox-rpi/redox-rpi/issues/14) deeper analysis: `ld.so` laid out static TLS like x86_64 on
  aarch64 (TLS aligned to more than 16 lost its initial values,
  initial-exec TLS of libraries pointed below the thread pointer). Fixed
  in experiment `tls-variant1` (branch `t14-tls-layout`), QEMU passes.
  None of today's packages uses the affected TLS forms. Also:
  sigaltstack is not implemented on aarch64.
- [#14](https://github.com/redox-rpi/redox-rpi/issues/14) the known failures left after PRs #62, #64 and #65
  (14 and 3 flaky), checked in QEMU on 2026-10-30:
  - Fixed on branches: `time/timer` (`timer-sigmask`, Pi test #86),
    `stdio/fscanf_offby1` and `stdio/ungetc_ftell` (`t-test-order`).
  - TLS layout: `tls_align` (dynamic and static), `tls_shared` and
    `stdio/mutex`. In `stdio/mutex` a second thread's `ftrylockfile`
    succeeds (15 of 15 runs): the owner check reads a thread-local
    thread-ID cache that a new thread does not get zeroed. `tls-variant1`
    on the current base (branch `t-tls-mutex`,
    `v0.3.0-dev-417-gd2482d8+x44a9`) fixes all four. `pthread/exit`
    fails on that image, but not because of the patch: kernel `0008`
    (ctx-leak fix, #31) makes a process whose last thread ends with
    `pthread_exit` report SIGKILL instead of 0. Every image with main
    merged has it; without `0008` the test passes 20 of 20.
    Fix: kernel experiment `self-exit-status` (branch `t-self-exit`,
    [#108](https://github.com/redox-rpi/redox-rpi/issues/108)); in QEMU
    10 of 10 runs exit 0 and the #31 leak stays fixed. Pi test 133
    passed (10 of 10 exit 0, no new suite failures).
  - `pthread/rwlock_randtest` does not hang: 1,000,000 lock operations
    take 11-17 s in QEMU, more than the verifier's 10 s. It passes.
  - `signals/kill-child`, `kill-permission`, `killpg0-self`: 45 of 45
    single runs pass; they only fail sometimes under the suite's load.
    With three busy loops on the 4 CPUs (QEMU, 2026-10-31,
    `v0.3.0-dev-574-g55e47fb+xb3d9`) also 45 of 45, so plain CPU load
    is not the trigger. Three full suite runs in one QEMU boot
    (same build) and the Pi suite runs of tests 129 and 133: none of the
    three failed. Not reproduced; they stay on the flaky list.
  - The five socket tests pass on the Pi (#68); QEMU has no network.
  - `redox_ring_ramfs`: not looked at yet.
  - For sshd, bash, nano and MQTT, next in this order: (1) the TLS layout
    (`tls-variant1`), since stdio locking across threads is broken without
    it; (2) the signal tests under load (job control in
    bash, child handling in sshd); (3) `rwlock_randtest`'s runtime, a note
    in the list only.
- No signal for exceptions on aarch64: relibc registered no exception
  handler, so a bad memory access ended the process without SIGSEGV, and
  sigaltstack was never used. Fixed in experiment `sigsegv` (branch
  `t-sigsegv`, relibc `exp/sigsegv`): `sigsegv`, `stackoverflow` and,
  with `sigentry-thread-signals`, `sigaltstack` pass in QEMU, no new
  failures. Pi test #68 passed (2026-10-29): 11 known failures, 0 new,
  14 fixed (also the socket tests, with network on the Pi). Draft PR #65
  makes it default relibc `0010` (stacked on #64).
- Timer signals: relibc's timer thread (`timer_settime`, `alarm`) did not
  block signals and could take the `SIGALRM` it sent to the process, so a
  thread waiting in `sem_wait` was not interrupted (`time/timer`). Fixed
  in experiment `timer-sigmask` (branch `t-timer-sigmask`), not
  aarch64-specific. Pi test #86 queued: branch `t-g3-pi` with this, dash,
  the test-order fix and the #68 fixes, QEMU 10 failures, none new.
- `stdio/fscanf_offby1` and `stdio/ungetc_ftell` were test bugs: they rely
  on x86_64's argument evaluation order. Fixed in relibc-tests patch
  `0004` (branch `t-test-order`).
- [#43](https://github.com/redox-rpi/redox-rpi/issues/43) lost wakeups in `thread::park` and `pthread_cond_*`: a kernel
  race, not relibc. `FUTEX_WAIT` compared the word before it queued itself
  under the lock `FUTEX_WAKE` takes, so a wake on another CPU in between
  was lost. Upstream fixed it on 2026-10-27 (`57ccc8f`, `5805059`), after
  our pin; experiment `futex-wait-lock` (branch `t-futex-wait-lock`)
  backports it. New test `pthread/cond_stress`: QEMU 3 of 21 runs lose a
  wakeup without, 0 of 41 with it. Pi test #98 queued.
- The static tests and statically linked C programs link the toolchain's
  `libc.a` (prefix sysroot, built 2026-10-24), not the relibc of the
  build: our relibc patches do not reach them. Fix for the tests (branch
  `t13-relibc-tests`): an experiment may replace an upstream recipe;
  `relibc-tests-bins` depends on relibc and links with
  `-B<sysroot>/lib/ -L<sysroot>/lib`. Static `tls_align` now passes with
  `tls-variant1`. Other static C recipes still need the same.

## Upstream pointers

Notes for the maintainer, who reports upstream himself. Each says what is
broken in upstream relibc or the kernel, how to see it and which of our
patches fixes it. The patch files are written against the pinned upstream
commits in `config/upstream.lock`.

- **Signals sent to one thread, aarch64** (relibc `0008`, `0009`; PR #64,
  [#4](https://github.com/redox-rpi/redox-rpi/issues/4)).
  - Where: `redox-rt/src/arch/aarch64.rs`, `__relibc_internal_sigentry`.
  - Broken: label `3` stores the signal number where the signal word
    belongs, so after one `pthread_kill` every signal of that thread stays
    blocked. Label `5` reads a realtime signal's number from the wrong
    register. Unlike x86_64, signals sent to a thread get no +64, so a
    realtime signal's sender comes from a stale `sa_tmp_rt_inf`. A standard
    process signal's sender is never stored, so `si_pid`/`si_uid` come
    from an earlier signal.
  - Reproduce: upstream's relibc tests `sigaction`, `sys_select/pselect`,
    `signals/sigpause-revert`, `signals/sigprocmask-3` and
    `signals/sigset-5` fail on aarch64 and pass with the patches. Our
    `sigregs thread` and `sigregs sender` (tools/sigregs) show it directly.
- **No signals for CPU exceptions, aarch64** (relibc `0010`; PR #65).
  - Where: `redox-rt/src/arch/aarch64.rs`, `redox-rt/src/signal.rs`.
  - Broken: relibc registers no exception handler on aarch64. A bad memory
    access, an undefined instruction or `brk` ends the process without a
    signal; the kernel prints `FATAL: Not an SVC induced synchronous
    exception`. `sigaltstack` is accepted but never used.
  - Reproduce: relibc tests `signals/sigsegv`, `signals/stackoverflow` and
    `sigaltstack` fail on aarch64; they pass with `0010`. `sigaltstack`
    also needs `0008`/`0009`.
  - Not done: `SEGV_ACCERR`, and SIGBUS for alignment faults. The kernel
    passes only the exception class, not the fault status code.
- **Lost futex wakeups, kernel** (experiment `futex-wait-lock`,
  [#43](https://github.com/redox-rpi/redox-rpi/issues/43)). No report needed: upstream kernel `57ccc8f` and `5805059`
  (2026-10-27) fix it, and our experiment backports them.
  - Broken in our pin: `FUTEX_WAIT` compared the word before it queued
    itself under the lock `FUTEX_WAKE` takes. With more than one CPU, a
    wake in between was lost (hangs in `thread::park`, `pthread_cond_*`).
  - Reproduce: relibc-tests patch `0005` (`pthread/cond_stress`), 3 of 21
    runs in QEMU `-smp 4`.
- **Static TLS laid out like x86_64, aarch64** (experiment `tls-variant1`,
  branch `t-tls-mutex`, [#14](https://github.com/redox-rpi/redox-rpi/issues/14); Pi test 129 passed,
  draft PR [#123](https://github.com/redox-rpi/redox-rpi/pull/123), stacked on #62).
  - Where: `ld.so` (`src/ld_so/`).
  - Broken: `ld.so` places the static TLS blocks of the program and the
    libraries loaded at start as on x86_64. aarch64 (TLS variant I)
    expects the first block at `align_up(16, p_align)` above the thread
    pointer, and initial-exec/static TLSDESC offsets above it. TLS aligned
    to more than 16 loses its initial values; a library's initial-exec TLS
    points below the thread pointer.
  - Reproduce: relibc-tests patches `0001`-`0003` (`tls_align`,
    `tls_shared`, `tls_probe`).
- **Timer thread takes its own SIGALRM** (experiment `timer-sigmask`; not
  aarch64-specific).
  - Where: `src/platform/redox/timer.rs`, `timer_routine`.
  - Broken: the thread that `timer_settime` and `alarm` start keeps the
    caller's signal mask, so it can take the SIGALRM it sends to the
    process. A thread waiting in `sem_wait` is then not interrupted.
  - Reproduce: relibc test `time/timer`, which fails some runs.
  - Fix: one line at the start of `timer_routine` blocks all signals, as
    glibc does.
- **Two tests rely on x86_64's argument order** (relibc-tests patch
  `0004`).
  - Where: `tests/stdio/fscanf_offby1.c`, `tests/stdio/ungetc_ftell.c`.
  - Broken: both call `getc()`/`fgetc()` and `ftell()` as arguments of one
    `printf`. gcc evaluates arguments right to left on x86_64 and left to
    right on aarch64.
  - Fix: the patch moves the calls into separate statements.
- Notes for the user's upstream reports on the x18 signal return
  ([#127](https://github.com/redox-rpi/redox-rpi/issues/127), bug),
  `clock_settime` ([#51](https://github.com/redox-rpi/redox-rpi/issues/51)),
  termios speeds ([#94](https://github.com/redox-rpi/redox-rpi/issues/94))
  and `madvise`/socket options
  ([#109](https://github.com/redox-rpi/redox-rpi/issues/109)), checked
  against current relibc master:
  [docs/upstream/relibc.md](../upstream/relibc.md).
- Already reported: patch `0004` (x1/x2 swap in the signal entry) as
  [relibc#309](https://gitlab.redox-os.org/redox-os/relibc/-/work_items/309).

## Later

- `clock_settime` / `settimeofday`, so `timesync` can set the clock through
  the normal API ([time.md](time.md)).
