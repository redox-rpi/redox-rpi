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
