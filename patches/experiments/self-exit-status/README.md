# Experiment: self-exit-status, exit status of a thread that ends itself

Kernel patch for [#108](https://github.com/redox-rpi/redox-rpi/issues/108),
on top of kernel `0008` (the ctx-leak fix for
[#31](https://github.com/redox-rpi/redox-rpi/issues/31)).

`0008` exits a thread that ends itself (relibc writes `ForceKill` to its
own `status` handle) after the syscall, by setting `being_sigkilled`. The
status read reported `being_sigkilled` as `ForceKilled`, and procmgr turns
that into SIGKILL when it is the last thread of the process. So a process
whose last thread called `pthread_exit` ended with SIGKILL instead of 0
(relibc test `pthread/exit`).

The patch marks such a thread with `exiting_itself` as well; the status
read reports it as `Dead` (exit status 0), and only real kills as
`ForceKilled`. The exit after the syscall, and with it the leak fix, stays.

```bash
scripts/build-image.sh --experiment self-exit-status
```

## Results

QEMU raspi3b (`-smp 4`), 2026-10-31, branch `t-self-exit` with
`t13-relibc-tests` merged, `--experiment relibc-tests --experiment
timesync --experiment self-exit-status`, build
`v0.3.0-dev-574-g55e47fb+xb3d9`:

| Check | Without the patch (main with `0008`) | With the patch |
|---|---|---|
| relibc suite | `pthread/exit` NEW: SIGKILL (`v0.3.0-dev-417-gd2482d8+x9903`) | 319 tests, 22 failures, all known, none new |
| `pthread/exit`, 10 runs | SIGKILL | 10 of 10 `rc=0` |
| `free` around 200 `dd` runs (#31 leak, about 76 KB per run before `0008`) | – | used 124576 KB before, 103524 KB after: no growth |

So the leak fix of `0008` stays and the exit status is 0 again.
