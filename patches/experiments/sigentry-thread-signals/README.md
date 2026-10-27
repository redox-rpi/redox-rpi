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

Still different from x86_64: it does not add 64 to the signal number for
signals sent to the thread, so `inner_c` treats them as sent to the
process.

```bash
scripts/build-image.sh --experiment sigregs-test --experiment sigentry-thread-signals
```

Test: `sigregs thread` (experiment [`sigregs-test`](../sigregs-test)).
Task T4 in [tasks.md](../../../docs/tasks.md).

## Results

QEMU, 2026-10-27: `sigregs thread` passes 2000 signals; without the patch
only 1 signal arrives and SIGUSR1 stays blocked. Details in the
[test reports](../../../docs/test-reports.md). On a real Pi 3B+: 10 of 10 runs of `sigregs thread` (1000 signals) pass.
