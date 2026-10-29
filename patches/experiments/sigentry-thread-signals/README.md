# Experiment: sigentry-thread-signals, signals sent to one thread

Relibc patch for the aarch64 signal entry (`__relibc_internal_sigentry`),
paths for signals sent to one thread (`pthread_kill`):

- **Standard signal (label `3`):** it picked the highest pending bit,
  blocked or not, and then stored the signal number as the new signal
  word. That cleared every allowed bit (all signals blocked) and set random
  pending bits. Now it picks from the pending allowed bits, like x86_64,
  and clears only that bit.
- **Realtime signal (label `5`):** it took the signal number from the
  wrong register, shifted by the loaded word instead of the signal index
  and stored the word as the sender info. Now it clears the signal's bit
  and stores 0 as the sender (`sender_infos` only has entries for the
  standard signals).

Patch `0002`, from the review against x86_64
([#4](https://github.com/redox-rpi/redox-rpi/issues/4)):

- **Signals sent to the thread get 64 added** to their number, as on
  x86_64. `inner` then takes the sender of a realtime signal from
  `sa_tmp_id_inf` (0) instead of a stale `sa_tmp_rt_inf` left by an
  earlier realtime signal to the process. The action lookup at label `2`
  masks the number.
- **Standard signal sent to the process:** the sender was loaded from
  `sender_infos` but never stored in `sa_tmp_id_inf`, so `si_pid` and
  `si_uid` came from an earlier signal. x86_64 stores it.

The rest of the selection matches x86_64, except that aarch64 picks the
highest pending signal and x86_64 the lowest (POSIX leaves the order open).
x86_64 reads `sender_infos` with the realtime index too (past its 32
entries); aarch64 stores 0 there instead.

Review result: both patches belong in the default relibc patches; they
stay here until the merge gives them a number
([patches/README](../../README.md#patch-numbers)).

```bash
scripts/build-image.sh --experiment sigregs-test --experiment sigentry-thread-signals
```

Test: `sigregs thread` and `sigregs sender` (experiment
[`sigregs-test`](../sigregs-test)). `sender` checks `si_pid` of a signal
from another thread and then from a child process; without `0002` the
second shows the first sender.

## Results

QEMU, 2026-10-27: `sigregs thread` passes 2000 signals; without the patch
only 1 signal arrives and SIGUSR1 stays blocked. Details in the
[test reports](../../../docs/test-reports.md). On a real Pi 3B+: 10 of 10 runs of `sigregs thread` (1000 signals) pass.
