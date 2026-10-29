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
  `t4-thread-signals`, patch `0002` fixes two more differences; QEMU
  passes, Pi test #37 queued.
- Static programs (`sigregs`) carry the relibc they were linked with;
  `build-image.sh` now relinks `sigregs` on every build (branch
  `t1-sigreturn`), before that a relibc-only change kept the old binary.
- [#13](https://github.com/redox-rpi/redox-rpi/issues/13) relibc tests on Redox: experiment `relibc-tests` (branch
  `t13-relibc-tests`) runs upstream's suite in QEMU. `scripts/relibc-tests.sh`
  builds, runs it and compares with `config/relibc-tests-expected.txt`
  (new failure: exit 1; fixed test reported). 319 tests, 25 known failures
  on main, 16 with `tls-variant1` and `sigentry-thread-signals`. Nightly
  unit for the build host ready (01:30 UTC, issue on a change), not
  installed: needs the branch on main.
- [#14](https://github.com/redox-rpi/redox-rpi/issues/14) deeper analysis: `ld.so` laid out static TLS like x86_64 on
  aarch64 (TLS aligned to more than 16 lost its initial values,
  initial-exec TLS of libraries pointed below the thread pointer). Fixed
  in experiment `tls-variant1` (branch `t14-tls-layout`), QEMU passes.
  None of today's packages uses the affected TLS forms. Also:
  sigaltstack is not implemented on aarch64.
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
