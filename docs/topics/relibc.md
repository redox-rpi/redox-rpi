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
  `sigentry-thread-signals` passes on the Pi; needs a review).
- [#40](https://github.com/redox-rpi/redox-rpi/issues/40) prebuilt programs (bash, vim, …): start fine on main in QEMU
  (experiment `prebuilt-programs`); the signal return overwrites x18,
  which these programs use, fixed by experiment `sigreturn-svc` ([#1](https://github.com/redox-rpi/redox-rpi/issues/1)). Next:
  recheck with that fix and on the Pi.
- [#13](https://github.com/redox-rpi/redox-rpi/issues/13) relibc tests on Redox in CI; [#14](https://github.com/redox-rpi/redox-rpi/issues/14) deeper analysis.

## Later

- `clock_settime` / `settimeofday`, so `timesync` can set the clock through
  the normal API ([time.md](time.md)).
