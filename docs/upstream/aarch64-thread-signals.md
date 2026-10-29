# aarch64: signals sent to one thread corrupt the signal mask

- **What breaks:** in `__relibc_internal_sigentry`, the path for
  signals sent to one thread (`pthread_kill`):
  - standard signal (label `3`): picks the highest pending bit, blocked
    or not, and stores the signal number as the new signal word, which
    blocks all signals and sets random pending bits;
  - realtime signal (label `5`): takes the number from the wrong
    register, shifts by the wrong value and stores the word as the
    sender info.
  Also (compared with x86_64): thread signals do not get 64 added, so a
  stale realtime sender is used, and the sender of a standard process
  signal is never stored, so `si_pid`/`si_uid` come from an earlier
  signal.
- **Reproduce:** `scripts/build-image.sh --experiment sigregs-test`,
  then `sigregs thread -n 2000`: "only 1 of 2000 signals arrived",
  "SIGUSR1 is still blocked afterwards". Linux cannot show it: the code
  is Redox-specific.
- **Upstream code:** [`redox-rt/src/arch/aarch64.rs`](https://gitlab.redox-os.org/redox-os/relibc/-/blob/c303135936e6a7e77812f371b3b9d0aa468ec3bc/redox-rt/src/arch/aarch64.rs#L264)
  labels `5` (line 264) and `3` (line 289).
- **Our patch:** experiment
  [`sigentry-thread-signals`](https://github.com/redox-rpi/redox-rpi/tree/t4-thread-signals/patches/experiments/sigentry-thread-signals)
  (`0001` labels 3 and 5, `0002` the +64 and the sender).
- **State:** passes on the Pi with `0001` (10 of 10); `0002` in Pi
  test plan #37.
