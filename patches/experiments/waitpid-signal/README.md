# Signals reach a process in waitpid() (#387, experiment `waitpid-signal`)

Build with experiment `sigcont-restart` (its relibc `0001` comes first;
this patch applies on top) and `tcc` for the check:
`--experiment tcc --experiment sigcont-restart --experiment waitpid-signal`.

## What was wrong

In the #386 check, a process blocked in `waitpid()` caught no SIGUSR1 at
all, out of thousands. The kernel and procmgr side work: `kill()` sets
the pending bit and interrupts the thread, the kernel sends procmgr a
Cancel for the waiting request, procmgr answers `ECANCELED`, and the
caller gets `EINTR`. A single case in a fresh process shows that: the
signal is caught at 0.3 s.

The signals got lost **after a storm**: one SIGUSR1 stayed pending, not
blocked, and was never delivered. From then on procmgr dropped every
`kill()` of SIGUSR1 as "already pending" (idempotent signals) and sent no
interrupt, so the process caught nothing for the whole next wait.

**Cause:** procmgr's interrupt only wakes a *blocked* thread. When the
signal comes while the thread runs with delivery inhibited (inside
redox-rt's `wrapper()`, between two calls, or in the trampoline handling
the signal before), the interrupt wakes nothing. The thread then blocks
in its next call with the signal pending, and no second interrupt comes.
`sigpending()` does not show it: it reports only *blocked* pending
signals.

## Fix

relibc patch
[`0001`](relibc/0001-redox-rt-deliver-a-pending-signal-before-and-after-a.patch):
`wrapper()` enters the signal trampoline when a signal the thread allows
is pending, right before the blocking call and again after a call that
succeeded (new `currently_pending_unblocked()` in `signal.rs`). A signal
that arrives in the last moment before the thread blocks still waits
until the call returns, but no longer: it is no longer left pending, so
later signals are not dropped.

Not done: closing that last window needs the kernel to check for pending
signals before a scheme call blocks (`UserInner::call_inner`). That would
change every scheme call, so it is left for a follow-up.

## Check

`ion /usr/share/waitpid-signal/check.ion` compiles `wpsig.c`. The program
waits in `waitpid()` for a child that exits after 1.5 s, while a forked
sender sends SIGUSR1 once at 0.3 s or as a storm (every 0.2 ms for
0.5 s), with and without `SA_RESTART`. Each case first in a fresh
process, then all four in one process. Wanted per case:

- the signal caught while waiting (first at < 1000 ms);
- the child reaped with status 7;
- `EINTR` only without `SA_RESTART`;
- no SIGUSR1 left pending afterwards (seen by blocking it for a moment).

## Results

QEMU, two runs each:

| | before (`v0.5.0-dev-187-gfa94267c`) | after (`v0.5.0-dev-188-g3c13d3f2`) |
|---|---|---|
| storm cases | SIGUSR1 LEFT PENDING in 2 of the 4 storm cases of each run | none left |
| one process, after the storms (earlier check version, `dev-184`) | SA_RESTART cases caught 0 | caught 1 and 42–909 |
| verdict | `WPSIG: FAIL`, both runs | `WPSIG: pass`, both runs |

The #386 check on the fixed image: the `waitpid` lines now catch signals
too (SA_RESTART 3062 caught, without SA_RESTART 5 of 5 EINTR, before 0),
`SIGCONT-REPRO: pass`.

Built with `--allow-dirty` for these runs only because of
[#393](https://github.com/redox-rpi/redox-rpi/issues/393) (base `0089`
does not `git apply` after `0085`). The Pi image `v0.5.0-dev-188-g3c13d3f2+xc1ab` is a clean build (fresh base checkout); `WPSIG: pass` twice on it in QEMU too.
