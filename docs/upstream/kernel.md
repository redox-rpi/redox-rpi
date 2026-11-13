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

## A killed thread does not cancel its pending scheme call (bug)

Our [#401](https://github.com/redox-rpi/redox-rpi/issues/401). Checked against the kernel we pin, `c951af42`.

- **What breaks:** after `nc` exits in a shell, the next line typed is
  lost. nc's stdin thread is blocked in a pty read when the process exits;
  the thread is SIGKILLed, but ptyd keeps the read and hands it the next
  line.
- **Why:** `src/scheme/user.rs`, `UserInner::call`: on a wakeup while the
  caller is `being_sigkilled`, `eintr_if_sigkill` returns EINTR before the
  `Opcode::Cancel` request is sent ("data loss doesn't matter"). The scheme
  never learns the caller is gone. For a pty it does matter.
- **Our fix:** default kernel patch `0054`: send the Cancel first, then
  return EINTR. ptyd (`scheme_utils::ReadinessBased`) answers it with
  ECANCELED and drops the read; the kernel then removes the request because
  the caller's context is gone.
- **Tested:** QEMU raspi3b over devsh: lost line 3 of 3 times before,
  0 of 3 after. Chaos `killio`, `procs` and `threads` rounds unchanged.


## A context switch and a TLB shootdown wait for each other (bug)

Our [#207](https://github.com/redox-rpi/redox-rpi/issues/207). Checked against the kernel we pin, `c951af42`.

- **What breaks:** the whole system freezes, with interrupts masked on
  every CPU. The only sign is one line, `TLB shootdown stalled on CPU #N:
  acknowledgements 0/1`, from the aarch64 `StallWatch` in
  `Flusher::flush`. Found by our chaos test: waves of up to 63 threads in
  one process that share a mutex and call `sched_yield`. It froze the Pi
  3B+ (test 237) and freezes QEMU raspi3b (4 CPUs) on the first or second
  run.
- **Why:** `src/sync/ordered.rs`, `RwLock::write_arc` (about line 557)
  waits with the spin crate's plain `write()`. Every other wait in that
  file loops on `try_*` and calls `PercpuBlock::maybe_handle_tlb_shootdown()`.
  The context switch takes context locks with `write_arc`
  (`direct_switch_to`, `switch_inner`, …) with interrupts masked. When the
  CPU holding the lock is in `Flusher::flush` waiting for this CPU's
  shootdown ack, neither CPU can go on. The QEMU monitor showed exactly
  that: one CPU in `Flusher::flush`, the CPU that owed the ack in
  `direct_switch_to` → `write_arc` → `spin::RwLock::try_write_internal`,
  its `wants_tlb_shootdown` still set.
- **Reproduce:** our `chaos` tool (`tools/chaos`, C):
  `chaos --kind threads --round-seed 0x4e3014eeaaa65f76`, or a loop of
  `chaos --kind threads --rounds 3`, on 4 CPUs.
- **Our fix:** default kernel patch `0052` (was `0042` in experiment
  `kernel-switch-shootdown`).
  `write_arc` loops on `try_write` and calls `maybe_handle_tlb_shootdown()`
  while it waits. About 15 lines.
- **Tested:** QEMU raspi3b. Without the fix, the `threads` loop froze in its
  first iteration; with it, 200 iterations (600 rounds) passed. Real Pi
  3B+: it helps, but the Pi still froze, more rarely, in tests 238 and 240
  (once in 7 runs) without a stall line, so there may be a second, rarer
  cause (our #207 stays open). Test 241, with extra lock-wait watches, ran
  1360 rounds without a freeze.
- **Second part (our kernel `0053`):** without the watches the Pi froze
  again (row 379, 2 in 3 loops). The watches only slow each spin, so this
  looks like a livelock on the lock word: spin 0.9.8 `try_read` adds a
  reader before it checks for a writer and takes it back, so spinning
  readers keep a writer or an upgrade (which needs zero readers) from
  ever succeeding. `0053` backs off exponentially in every wait in
  `src/sync/ordered.rs`, and `RwLockUpgradableGuard::upgrade`, which also
  spun inside spin without handling TLB shootdowns, now loops on
  `try_upgrade` like the others. Pi result: our row 384.

## aarch64: no cache maintenance when user code becomes executable (bug)

Found in the kernel `c951af42` (the commit we pin) and still in upstream
master `1a1295b`: the only `ic ialluis` there is in the start-up of a
secondary CPU (`src/arch/aarch64/device/cpu/boot.rs`).

- **What breaks:** on a real Cortex-A53 (Raspberry Pi 3B+), code that was
  written as data runs stale. Our stress test made a page executable with
  `mprotect` after writing new code into it, and the old code ran in about
  a third of the rounds on a fresh page and about half on a reused one. QEMU
  does not model caches, so it never shows this.
- **Why:** instruction fetch does not see what is still in the data cache.
  The data cache has to be cleaned to the point of unification and the
  instruction cache invalidated before such code runs. Nothing does that
  for user code: not `mprotect` to executable (`ld.so` after relocation),
  not exec (`redox-rt` writes the program through a mapping in the calling
  process, then the `proc:` address space switch starts it), not an `fmap`
  with `PROT_EXEC`, and not a page fault that copies a frame into an
  executable grant. The Cortex-A53's instruction cache is VIPT and may
  alias, so it needs `ic ialluis` from EL1; `ic ivau` from user space would
  not cover the kernel's or another process's mapping.
- **Reproduce:** a Pi 3B+ (or any real aarch64 board) with our
  `tools/icachetest`: `icachetest 10000`; its `new-page` and `same-page`
  tests count rounds that ran old code.
- **Our fix:** experiment `icache-sync` (PR #148), kernel patches `0001`
  (sync on `mprotect` to executable and at the exec address space switch)
  and `0002` (executable `fmap`, and page faults that copy a frame or take
  one from a scheme). Both apply to upstream master `1a1295b` as they are.
- **Tested:** Pi 3B+ test 162 ([report](https://github.com/redox-rpi/redox-rpi/blob/main/https://github.com/redox-rpi/pi-tests/blob/main/reports/archive-before-v0.4.0-dev.md#2026-10-31-v030-dev-801-gea8b5c2-without-x8a5e-and-with-icache-sync-x9080-instruction-cache-stress-test)): without the patches
  3146 and 3244 of 10000 `new-page` rounds and 5310 and 5349 `same-page`
  rounds ran stale code; with them 0 in every test on both boots, 0 of 1000
  failed starts, and no measurable cost (the `ls` loop 116.8 s with against
  118.9 s without). QEMU: no exception, the stress test passes.
