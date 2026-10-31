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
  #37 and #68 pass. Draft PR #64 makes both patches default relibc
  `0008` and `0009` (stacked on #62).
- Static programs (`sigregs`) carry the relibc they were linked with;
  `build-image.sh` now relinks `sigregs` on every build (branch
  `t1-sigreturn`), before that a relibc-only change kept the old binary.
- [#109](https://github.com/redox-rpi/redox-rpi/issues/109) calls sshd makes on every connection logged
  `TODO` lines: `madvise` hints now succeed quietly, `getsockopt` of `IP_TOS`
  and `TCP_NODELAY` reports 0 (netstack's state) instead of ENOSYS, and
  `setsockopt` of both succeeds without a log line. relibc `0045` in
  experiment `sshd`, branch `relibc-quiet`. QEMU: key logins work, no TODO
  lines; Pi test #132 queued. Switching Nagle off needs netstack support.
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

## Later

- `clock_settime` / `settimeofday`, so `timesync` can set the clock through
  the normal API ([time.md](time.md)).
