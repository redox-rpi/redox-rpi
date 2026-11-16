# A signal the program does not catch gives no EINTR (#386, experiment `sigcont-restart`)

`timeit` (fork, exec, `waitpid`) printed `timeit: Interrupted system
call` twice in one Pi run (pi-tests row 359) and left its child running,
although it catches no signal. POSIX allows `EINTR` only when a caught
signal without `SA_RESTART` interrupts the call.

## Cause

In redox-rt, a blocking call that returns `EINTR` (`wrapper()` in
`sys.rs`) enters the signal trampoline and restarts the call only if
`last_sig_was_restart` is set afterwards. Two cases left it `false`
although no handler without `SA_RESTART` ran:

- **SIGCONT with its default action.** No handler runs, but
  `shall_restart` came from the default action's flags, which have no
  `SA_RESTART`. A `read()` on a pipe returned `EINTR` for every SIGCONT.
  Shells send SIGCONT to the jobs they start, which fits "right after a
  tcc call".
- **No signal at all.** When the entry finds nothing pending (label `7`
  in the aarch64 entry: the signal was taken meanwhile, or the `EINTR`
  came without one), the trampoline returns without touching
  `last_sig_was_restart`. It keeps the value from the signal before,
  for example `false` after a default SIGCONT. `waitpid()` returned
  `EINTR` with no signal caught.

## Fix

relibc patch
[`0001`](relibc/0001-redox-rt-a-signal-without-a-handler-restarts-the-int.patch):
- `wrapper()` sets `last_sig_was_restart = true` before it enters the
  trampoline, so only a handler without `SA_RESTART` clears it;
- the trampoline treats default SIGCONT as restarting.

Upstream relibc has the same code; a report is the user's to post.

## Check

`ion /usr/share/sigcont-restart/check.ion` (needs experiment `tcc`)
compiles `sigcont.c`. A forked sender sends a signal to the program every
0.2 ms while it blocks in `waitpid()` for a child, or in `read()` on a
pipe the child writes after 0.5 s; 5 rounds each:

| line | want |
|---|---|
| SIGCONT, default action | 0 EINTR |
| SIGUSR1 with `SA_RESTART` | 0 EINTR |
| SIGUSR1 without `SA_RESTART` (the control: the signals do arrive during the call) | EINTR (`read` only) |

The `waitpid` control is shown but not counted: a process blocked in
`waitpid()` gets no signal at all yet,
[#387](https://github.com/redox-rpi/redox-rpi/issues/387).

## Results

QEMU, two runs each:

| line | before (`v0.5.0-dev-147-g6a189e63`) | after (`v0.5.0-dev-148-g595010cb`) |
|---|---|---|
| waitpid, SIGCONT default | 1 of 5 EINTR, both runs | 0, 0 |
| waitpid, SIGUSR1 SA_RESTART | 1 of 5 EINTR with 0 caught, both runs | 0, 0 |
| read, SIGCONT default | **5 of 5 EINTR**, both runs | 0, 0 |
| read, SIGUSR1 SA_RESTART | 0, then 1 of 5 EINTR | 0, 0 (1131, 1398 caught) |
| read, SIGUSR1 no SA_RESTART | 5 of 5 EINTR | 5 of 5 EINTR (control holds) |
| verdict | `SIGCONT-REPRO: FAIL` | `SIGCONT-REPRO: pass`, no panic |

Pi 3B+, row [361](https://github.com/redox-rpi/pi-tests/blob/main/reports/361.md),
2026-11-12, on the PR's own image `v0.5.0-dev-148-g595010cb+x0016`:
**pass**. Both runs `SIGCONT-REPRO: pass`, 0 of 6 lines wrong: default
SIGCONT and SIGUSR1 with SA_RESTART 0 of 5 EINTR for waitpid and read;
the read control 5 of 5 EINTR (515 and 589 caught); waitpid without
SA_RESTART 0 of 5, 0 caught (#387). Ten `timeit tcc` runs: no EINTR,
100.0–102.8 ms. pi-test 6/6, no panic, no stall.
