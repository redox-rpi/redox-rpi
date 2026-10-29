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
