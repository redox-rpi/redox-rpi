# Experiment: futex-wait-lock, no lost futex wakeups

Backport of two upstream kernel commits to our pinned kernel
(`config/upstream.lock`), for `FUTEX_WAIT` in `src/syscall/futex.rs`:

- `0001` = upstream `57ccc8f` "futex: Fix context-wide deadlock"
- `0002` = upstream `5805059` "futex: Do not return early without rollback"

Both are from 2026-10-27, after our pin; they go away when the pin moves
past them.

The bug in the pinned kernel: `FUTEX_WAIT` compared the futex word with
the expected value, blocked the context and only then took the `FUTEXES`
lock to queue itself. `FUTEX_WAKE` takes that lock and wakes the queued
contexts. A wake on another CPU between the comparison and the queueing
found no waiter and returned 0, and the waiter went to sleep although the
word had changed: a lost wakeup, the thread sleeps forever. Every user of
futexes can hit it with more than one CPU: relibc's mutexes, condition
variables (`pthread_cond_*`), semaphores, and so Rust's `thread::park`,
`Mutex` and `Condvar`
([#43](https://github.com/redox-rpi/redox-rpi/issues/43)).

Upstream now queues the context first, then reads the word again under
the lock and rolls back (`EAGAIN`) if it changed, so a waker either sees
the queued waiter or changed the word before the second read.

```bash
scripts/relibc-tests.sh --experiment futex-wait-lock
```

Test: `pthread/cond_stress` in the relibc test suite (relibc-tests patch
`0005`, branch `t-cond-stress`): two threads hand a turn back and forth
through a condition variable and through a parker built like Rust's,
every wait with a 1 s timeout; a timeout while the condition already
holds is a lost wakeup and the test exits 1.

## Results
