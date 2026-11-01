# Experiment: sched-vtime-clamp, scheduler virtual time that cannot run away

Kernel fix for the netget download stall
([#157](https://github.com/redox-rpi/redox-rpi/issues/157),
[#151](https://github.com/redox-rpi/redox-rpi/issues/151)): during a long
download, a user context (netstack, dwc2d, sdhostd) stays runnable in its
CPU's run queue but is never picked; that CPU only runs idle, and everything
waiting on that context stops.

What the `pi-heartbeat` dumps showed (Pi tests 183–188):
- The stuck context sits in the queue as `Rqa` (runnable, queued, active),
  and `select_next_context` skips it without a lock or Skip reason.
- Its queue entry has `vtime` = `u64::MAX` (188). Other queues' `v` and
  other entries sit just below 2^64.
- Virtual time goes above 2^62 in **every** download, about 40 s in, at all
  five places that change it. Runs only stall when such an entry is not
  cleared before the pick.

The cause, in the upstream EEVDF scheduler (`src/context/switch.rs`): each
CPU has its own virtual time `v`, and the `v` of the four CPUs drift far
apart (96422 against 1.1e9 in 187). A woken context gets
`vtime.max(run_queue.v)`, so a context that last ran on a busier CPU arrives
far ahead of a quiet CPU's `v`. If it is the only one there,
`v = ineligible_min_vtime` moves that queue's `v` up to it. Work stealing
moves a context with `vtime = own_v + (vtime - target_v)`, which adds the
gap once more. Values roughly double with each round (188: old vtime
3.39e18, own `v` 2.04e18, new vtime 5.43e18) until they wrap past
`u64::MAX`. After that, `vtime <= v` and `vtime > ineligible_min_vtime`
compare wrapped numbers, and an entry can stay unpicked for good.

The fix: a context that joins a run queue (on wakeup in `schedule()`, and
when stolen) gets at most one slice of virtual time ahead of that queue's
`v`. A context that is behind keeps its credit as before. With that bound,
`v = ineligible_min_vtime` moves `v` by at most one slice, and stealing
cannot add up gaps.

```bash
scripts/build-image.sh --experiment sched-vtime-clamp
```

It applies with and without `pi-heartbeat`, in either order. With
`pi-heartbeat` the `[hbv …]` lines (a value above 2^62) should no longer
appear.

## Results

QEMU raspi3b, 2026-11-01:
- Download test build `v0.3.0-dev-890-gfa908ca+x578b` (branch `t157-vc-usb`,
  with `pi-heartbeat`): the 382 MB download of Pi test 172 over QEMU's USB
  network completes in 110 s, with no `[hbv]` line. The same run without the
  fix (`v0.3.0-dev-889-g7349b1c+x741f`) also completes without `[hbv]`, so
  QEMU does not show the wrap; only the Pi can confirm the fix (row 189).
- relibc suite `v0.3.0-dev-916-gcb7d599+xbadd` (`relibc-tests`, `timesync`,
  `futex-wake-efault`, `sched-vtime-clamp`): 18 failed, 17 known, the same
  result as without this patch (the new one, `pthread/exit` SIGKILL, is an
  old intermittent failure).
