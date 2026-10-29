# Experiment: timer-sigmask, the timer thread blocks all signals

`timer_settime` (and `alarm`, which uses a timer) starts a thread that
sends `SIGEV_SIGNAL` signals to the process. The thread inherited the
caller's signal mask, so it could take the signal itself. A thread
waiting for the signal in `sem_wait` or `pause` was then not interrupted:
it was woken by what the handler did on the other thread. Which thread got
the signal varied from run to run.

Relibc patch `0001` (branch `exp/timer-sigmask` in redox-rpi/relibc):
`timer_routine` blocks all signals, as glibc does in its timer thread.
Not aarch64-specific.

A probe (not kept) before the fix, `alarm(1)` with a handler that posts a
semaphore: `read` -1/EINTR, `sem_wait` 0 (the timer thread took the
signal), then -1/EINTR in the next try.

```bash
scripts/relibc-tests.sh --experiment timer-sigmask
```

## Results

QEMU raspi3b, `scripts/relibc-tests.sh --experiment timer-sigmask`,
`v0.3.0-dev-222-g398fbc3+xc933` (2026-10-29), three runs: 23 failures,
`time/timer` passes every time (before: fails every time), none new.
`stdio/mutex` also passed in all three, but it uses no signals: it fails
or passes depending on the build (passed with the #4/#14 fixes too), so it
is marked flaky in the list rather than counted as fixed.
