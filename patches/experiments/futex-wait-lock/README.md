# Experiment: futex-wait-lock, no lost futex wakeups

Kernel patch for `FUTEX_WAIT` (`src/syscall/futex.rs`).

`FUTEX_WAIT` compared the futex word with the expected value, blocked the
context and only then took the `FUTEXES` lock to queue itself. `FUTEX_WAKE`
takes that lock and wakes the queued contexts. A wake on another CPU between
the comparison and the queueing found no waiter and returned 0, and the
waiter went to sleep although the word had changed: a lost wakeup. Every
user of futexes can hit it with more than one CPU: relibc's mutexes,
condition variables (`pthread_cond_*`), semaphores and so Rust's
`thread::park`, `Mutex` and `Condvar`.

The patch compares again while holding the `FUTEXES` lock and blocks and
queues the context under it, as Linux does under its hash bucket lock. A
waker that changed the word before is seen by the second comparison
(`EAGAIN`); one that comes later finds the waiter. The first comparison
stays as the fast path.

```bash
scripts/relibc-tests.sh --experiment futex-wait-lock
```

Test: `pthread/cond_stress` in the relibc test suite (relibc-tests patch
`0005`): two threads hand a turn back and forth through a condition
variable and through a parker built like Rust's, every wait with a 1 s
timeout; a timeout while the condition already holds is a lost wakeup.

## Results
