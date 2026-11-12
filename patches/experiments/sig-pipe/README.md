# Experiment: sig-pipe, a forked child gets its signals

For [#205](https://github.com/redox-rpi/redox-rpi/issues/205) (topics
relibc, stability), found by the chaos test (`chaos --kind signals`): a
process that loops on pipe read/write got none of the thousands of
`kill(pid, SIGUSR1)` its children sent, although every `kill()` returned 0.

**The cause:** it is not the pipe. Any process that was created by `fork()`
and did not `exec` afterwards never ran a signal handler, except while it
was blocked (a `usleep` returned EINTR and the handler ran). redox-rt's
`fork_inner` copies the address space inside `dup_into_upper("exclusive")`,
and `dup_into_upper` holds a `tmp_disable_signals()` guard. So the child's
memory is a snapshot with that guard taken: `disable_signals_depth` 1 and
`INHIBIT_DELIVERY` set in its signal control. The child starts at the fork
return path and never drops that guard, so delivery stays inhibited for the
rest of its life. The kernel's `signal_handler` returns early on
`INHIBIT_DELIVERY`. Only blocking calls that return EINTR deliver, through
`manually_enter_trampoline`.

Upstream relibc master (df6f5df, 2026-11-11) has the same code. Upstream
relibc issue
[#306 "Likely tmp_disable_signals leak"](https://gitlab.redox-os.org/redox-os/relibc/-/work_items/306)
(2026-09-05, open) describes the effect: signals inhibited outside relibc
code with the counter at 1. This is probably its cause.

**The fix** (`relibc/0001`): `fork_impl` notes the caller's
`tmp_disable_signals` depth before forking. The child sets the depth and
`INHIBIT_DELIVERY` back to it before it restores its signal mask, and the
mask restore delivers anything already pending.

**Not fixed here:** a thread that is running (not blocked) gets a signal
only at its next context switch, because procmgr's `Interrupt` only wakes
blocked threads. A busy loop sees about 30 signals per second, while Linux
or macOS see hundreds of thousands. POSIX allows this, but it is slow.

**Check:** build with `--experiment tcc --experiment sig-pipe`, then

    ion /usr/share/sig-pipe/check.ion

compiles `sigrepro.c` with tcc and lets a child send `SIGUSR1` as fast as
it can to a receiver that loops in user code, `getpid()`, `usleep()` and
pipe read/write, once as a forked child and once exec'd. Each of the 8
receivers must get at least one signal: `SIGREPRO: pass`. With experiment
`chaos` too, `chaos --kind signals --round-seed 0x218d87da9382bd2e` (the
seed from #205) must report more than 0 signals.

## Results

- **QEMU raspi3b, 2026-11-12, without the fix**,
  `v0.5.0-dev-102-g3d9b5bda+x800c`: forked receivers got 0 signals in user
  code, getpid and pipe I/O, and 1746 in usleep. Exec'd receivers got
  30/24/61/8.
- **QEMU raspi3b, 2026-11-12, with `0001`**,
  `v0.5.0-dev-103-g08bc7239+xb4a3` (timesync, tcc, sig-pipe, chaos): forked
  receivers got 29/27/1119/7 and exec'd 28/31/4568/8, so `SIGREPRO: pass`.
  `chaos --kind signals` with the #205 seed: 13 signals (before: 0); seeds
  1 and 2: 27 and 10. No `RELIBC PANIC`.
- **Real Pi 3B+, 2026-11-12**, pi-tests row 352
  ([report](https://github.com/redox-rpi/pi-tests/blob/main/reports/352.md)):
  the first run on `v0.5.0-dev-103-g08bc7239+xb4a3` hit the #151 network
  stall (devsh unreachable, no panic) and does not count. Rerun on
  `v0.5.0-dev-106-g0e7b4571+xb4a3`, which is this branch merged with the
  dwc2d fix #373 (dwc2d-no-sleep), same experiments: `SIGREPRO: pass`.
  Forked/exec'd receivers got user 20/23, getpid 24/24, sleep 51/42159
  and pipe 7/6 signals. `chaos --kind signals` with seeds 0x218d87da9382bd2e,
  1 and 2 got 7, 19 and 5 signals. No panic; pi-test passes. **Pass.**
