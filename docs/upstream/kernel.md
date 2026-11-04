# Kernel: pointers for upstream

Notes for the user, who reports to Redox. Checked against upstream on
2026-11-01: kernel `master` at `1a1295b`. (The kernel side of the netstack
leak is in [netstack.md](netstack.md).)

## A blocking pipe write returns after 64 KiB (bug)

Our [#158](https://github.com/redox-rpi/redox-rpi/issues/158).

- **What breaks:** a blocking `write()` to a pipe of more than the free
  space in the pipe (at most 64 KiB) returns a short count. POSIX says a
  blocking write returns the full count on normal completion; a short count
  is only for an interrupting signal or a non-blocking write. Programs that
  don't retry lose data: uutils `dd` (Redox' `dd`, coreutils 0.11.0) retries
  short writes only with `iflag=fullblock`, so `dd bs=1M count=16 | cmd`
  passes on 1 MiB of 16 and reports `0+16 records out`. Found in our Pi
  test 177.
- **Why:** `src/scheme/pipe.rs`, `kwrite` (line 320). It copies
  `min(MAX_QUEUE_SIZE - queued, len)` bytes (lines 343–346, queue size
  65536 at line 38) and returns that count right away (line 374). It waits
  on `write_condition` only when nothing fit (lines 379–387).
- **Reproduce:** any Redox (QEMU is enough):
  `dd if=/dev/zero bs=1M count=4 | wc -c` gives 262144 instead of 4194304,
  and dd says `0+4 records out`. With `iflag=fullblock` all bytes arrive.
- **Our fix:** kernel patch `0001` in experiment `pipe-write-all` (branch
  `pipe-write-all`, PR [#160](https://github.com/redox-rpi/redox-rpi/pull/160)). `kwrite` keeps a running total: after each
  copy it wakes the reader and, if bytes are left and the write is
  blocking, waits on `write_condition` and goes on. It returns the total
  when all bytes are in, or the bytes written so far when a signal
  interrupts the wait, the reader goes away, or the user buffer faults
  partway. A non-blocking write is unchanged. About 30 lines in that one
  function.
- **Also not POSIX, not fixed:** writes of up to `PIPE_BUF` (4096) bytes
  are not atomic when the queue has less room than that.
- **Tested:** QEMU: `dd bs=1M count=16 |` passes 16777216 bytes
  (`16+0 records out`), `dd … | sha256sum` matches the file, and
  `dd … | head -c 100` ends with `Broken pipe` instead of hanging. Pi 3B+ test 180 passed.

## The scheduler's virtual time runs away and wraps (bug)

Our [#157](https://github.com/redox-rpi/redox-rpi/issues/157) and [#151](https://github.com/redox-rpi/redox-rpi/issues/151).

- **What breaks:** under load across CPUs (a long network download to the SD
  card on the Pi 3B+, 4 CPUs), a runnable context (netstack, dwc2d,
  sdhostd) stays in its CPU's run queue and is never picked again. That CPU
  only runs idle, and everything waiting on that context stops for good.
  Found in our Pi tests 179–188: 1 to 3 of 3 downloads stalled.
- **Why:** `src/context/switch.rs`, the EEVDF scheduler. Each CPU's run
  queue has its own virtual time `v`, and the `v` of the CPUs drift far
  apart (96422 against 1.1e9 in one of our dumps).
  - A woken context gets `vtime.max(run_queue.v)` (line 394). A context
    that last ran on a busier CPU therefore arrives far ahead of a quiet
    CPU's `v`.
  - If no context is eligible, `select_next_context` sets
    `contexts_data.v = ineligible_min_vtime` (line 689), so that queue's
    `v` jumps up to the newcomer.
  - Work stealing sets `vtime = own_v + (vtime - target_v)` (lines
    868–869), which adds the gap once more.
  - Values roughly double with each round (one steal: old vtime 3.39e18,
    own `v` 2.04e18, new vtime 5.43e18) until they wrap past `u64::MAX`.
    After that `*vtime <= contexts_data.v` and
    `*vtime > ineligible_min_vtime` compare wrapped numbers. In the stall
    we saw an entry with `vtime == u64::MAX` that its CPU never picked.
- **Reproduce:** needs real multi-CPU load; our QEMU `raspi3b` runs do not
  show it. A kernel check that prints the first `v`/`vtime` above 2^62
  fires in every 382 MB download on the Pi 3B+, about 40 s in (Pi test
  188).
- **Our fix:** kernel patch `0012` (experiment `sched-vtime-clamp`, PR
  [#168](https://github.com/redox-rpi/redox-rpi/pull/168)). A context that
  joins a run queue, on wakeup in `schedule()` and when stolen, gets at most
  one slice of virtual time ahead of that queue's `v`
  (`(BASE_SLICE_TICKS * SCALE) / weight`). A context that is behind keeps
  its credit. About 15 lines.
- **Not fixed, seen alongside:** the per-queue `total_weight` drifts: one of
  our CPUs showed `total_weight` 582656 with an empty queue and growing. A
  context made active in one CPU's queue that blocks on another CPU is
  subtracted from the wrong queue. It affects fairness, not the stall.
- **Tested:** Pi 3B+ test 189: no `v`/`vtime` above 2^62 in 6 downloads
  (largest 3.4e13; before, 5e18–1.8e19 in every download), no stuck
  context. One of the 6 downloads stopped from a separate USB controller
  fault. QEMU: relibc suite unchanged, the download completes.

