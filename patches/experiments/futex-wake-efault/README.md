# Experiment: futex-wake-efault, no panic when a thread is joined early

relibc fix for the `brk` seen in `pthread/rwlock_randtest` on the real Pi
(tests 176 and 178, [#14](https://github.com/redox-rpi/redox-rpi/issues/14)):
one run in about 40 printed

```
RELIBC PANIC: panicked at src/sync/mod.rs:103:54:
called `Result::unwrap()` on an `Err` value: Errno(14)
```

and the thread died at `ELR 0x516C4`, the `brk #1` at the end of
`relibc_panic` (libc.so at 0x1000). The run still returned 0, because only
the exiting thread died.

The cause: on thread exit `Waitval::post` (`src/sync/waitval.rs`) stores the
return value with `state = 1`, then calls `futex_wake` on `state`. The
joining thread can see `state == 1` without waiting, return from
`pthread_join` and free the exiting thread's `Pthread` before the wake. The
kernel then finds no mapping for the address and returns `EFAULT`, and
`futex_wake_ptr` (`src/sync/mod.rs`) unwraps it. `exit_current_thread`
already notes that `waitval.post()` may unmap the TCB.

The fix: `futex_wake_ptr` returns 0 woken threads when the wake fails,
instead of panicking. No thread can wait on memory that is no longer
mapped. Not aarch64-specific; it is a race, so it shows up more often on the
real Pi than in QEMU (0 in 40 QEMU runs).

```bash
scripts/build-image.sh --experiment relibc-tests --experiment futex-wake-efault
```

## Results
