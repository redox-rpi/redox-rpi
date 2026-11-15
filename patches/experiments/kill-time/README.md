# Experiment: kill-time, how fast signals end a process

Test only, for [#405](https://github.com/redox-rpi/redox-rpi/issues/405)
(Pi row 399: `kill` did not end Sopwith, and `kill -KILL` seemed to take
5 s). Build with experiment `tcc`; `ion /usr/share/kill-time/check.ion`
compiles `killtime.c` and runs it. The regression set runs it as check
`kill-time`.

`killtime` starts one child per case (fork and exec), waits 300 ms, sends
SIGTERM or SIGKILL and measures how long `kill()` takes and when
`waitpid()` sees the child gone. A case passes when `kill()` returns within
0.5 s and the child is gone within 1 s. Cases: `sleep`, `sleeploop` (1 ms
sleeps), `spin`, `read` (empty pipe), `audio` (blocking writes to
`/scheme/audio`), `thr-read` and `thr-audio` (the main thread sleeps, a
worker with SIGTERM blocked, as SDL's threads have, is blocked in a call),
`main-masked` (only a worker can take SIGTERM) and `handler` (SDL's own
SIGTERM handler: a flag the main loop polls). `./killtime <case>` runs one.

`killwatch.c` (`tcc -o kw /usr/share/kill-time/killwatch.c`, then
`kw <pid> [signal]`) sends a signal and prints when the target's threads in
`/scheme/sys/context` change or go, for programs like Sopwith.

## What #405 turned out to be (QEMU, 2026-11-16)

- All 17 cases pass: SIGTERM and SIGKILL end a process within 15 ms,
  including a thread blocked in a write to `/scheme/audio`. relibc,
  procmgr, the kernel and audiod are not the cause.
- **SIGTERM:** SDL2 turns SIGTERM (and SIGINT) into an `SDL_QUIT` event, and
  Sopwith 2.9.0 has no `case SDL_QUIT` in its event loop, so it ignores it
  (on Linux too; upstream master is the same). Fixed by
  `orbital-apps/recipes/games/sopwith/0001-*.patch`;
  `pwm-audio/qemu-sdl-check.sh` checks it.
- **SIGKILL "5 s":** `killwatch` sees Sopwith's threads gone 30–45 ms after
  `kill -KILL`. The 5 s were the test harness: when a background job ends by
  a signal, Ion prints `pipeline execution error: process (N) ended by
  signal SIGKILL` and drops the rest of the current command line, the end
  marker of `pi_test.Shell.run` included, which then waited 5 s for a
  second prompt (fixed in `scripts/pi-test.py`, its own PR).
