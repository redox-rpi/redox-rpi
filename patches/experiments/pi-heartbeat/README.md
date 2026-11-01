# Experiment: pi-heartbeat, does the kernel still run?

Diagnostic kernel patch for the whole-machine freeze in
[#14](https://github.com/redox-rpi/redox-rpi/issues/14) (`pthread/rwlock_randtest`
on the real Pi, tests 166, 170, 173). Not meant for main.

Every 6000 timer interrupts (60 s at 100 Hz; 10 s until test 176) each CPU writes
`[hb <cpu> <n> s<sched> w<switch> i<idle> p<pit>]` to the serial console, straight to the PL011 data
register and without the COM1 lock (`uart_pl011::raw_write`). Output can
interleave with other console output.

The counters: `s` calls of `schedule()`, `w` real switches to another
context and `i` idle results, all since this CPU's last beat; `p` its
`pit_ticks`, timer ticks since its last `schedule()` (normally below 3).
A CPU that beats with `s0` and a growing `p` takes interrupts but no
longer schedules: the starvation seen in #174 (a runnable `netget` that
never ran).

What a freeze then shows in the serial log:
- all CPUs keep writing: the CPUs take timer interrupts; the freeze is
  in the scheduler or a lock with interrupts on (a deadlock), or in user
  space / devsh.
- some CPUs stop: those CPUs hang with interrupts off (a spin lock held
  in interrupt context, or a fault loop).
- all stop at once: the whole machine (interrupt controller, bus, memory)
  or every CPU in the same lock.

Stall watch (after test 176, where the 10 s beat may have hidden the
freeze): on every timer interrupt each CPU checks the others. When another
CPU took no timer interrupt for 2 s it writes `[hb stall <cpu> seen-by
<cpu> t<ticks> s<sched>]` once; when another CPU still ticks but ran no
`schedule()` for 2 s, `[hb nosched ...]`. Quiet otherwise.

Context dump (after test 179, where netget, devsh, sdhostd and redoxfs all
stopped while every CPU kept beating with `w0`): CPU 0 writes the context
list, `[hbctx begin beat <n> <why>]`, then one line per CPU
`[hbcpu <cpu> w<switches since the last beat> q<run queue length> sl<n> sa<n> sr<n>]`, then one
line per context `[hbctx <pid> <name> <stat><flags>[ sys] cpu<n> <reason>]`,
then `[hbctx end]`.
`<stat>` is R runnable, S sleeping with a timeout, B blocked, H hard-blocked
(with its reason), Z dead. `<flags>`: `+` running, `c` currently_scheduled,
`q` in a run queue (queue_key set), `a` active; `sys` inside a syscall.
`<reason>` is what the context blocked on (`futex`, `nanosleep`,
`UserInner::call` for a scheme call). `<why>` is `baseline` (once at beat 2),
`few-switches` (all CPUs together made fewer than 100 context switches since
CPU 0's last beat) or `runnable-on-idle-cpu` (a CPU made no switch since the
last beat while a runnable, not running user context has that CPU as its
`cpu_id`). After the baseline it writes at most 3 lists. A runnable context
without `q` points to a lost wakeup; one with `q` or `c` that its CPU never
picks points to the run queue selection. `sl`, `sa` and `sr` count, since boot, how
often that CPU's `select_next_context` passed over a queued context because
its context lock was taken (`sl`), its address space lock was taken (`sa`),
or `update_runnable` said Skip (`sr`: running elsewhere, currently scheduled,
affinity). The counter that grows from one dump to the next names the reason
a queued context is not picked. `sv` counts entries skipped by the vtime
filter, `ss` entries that are the idle or the current sched context. After
the CPU lines each CPU's real run queue follows: `[hbq <cpu> v<v> n<entries>
tw<total weight>]`, then per entry (at most 8) `[hbq <cpu> vt<vtime>
id<ctxt id> <pid> <name> own|stale]` (`stale`: the context's own queue key
is a different entry). Besides `baseline` and the two triggers, a `timed`
list comes every 2nd beat (about every 2 min), at most 12 times.

The scheduler also checks virtual time at the five places that change it:
the wakeup in `schedule()` (1), the previous context's account in
`select_next_context` (2), work stealing (3), `v += v_advance` (4) and
`v = ineligible_min_vtime` (5). The first value above 2^62 at each place is
kept with its inputs and written at the next beat as `[hbv <place> cpu<n>
<value> <a> <b> <c>]`: 1 old vtime, queue v, context id; 2 old vtime (or the
penalty), elapsed ns (or the weight), queue v; 3 old vtime, the other
queue's v, own v; 4 old v, elapsed ns, total weight; 5 old v, whether the
previous context was runnable, elapsed ns.

Kernel code on aarch64 runs with interrupts masked, so CPUs that deadlock
while spinning take no timer interrupt and write no beat at all (Pi test
199: no line after the freeze). So the spin loops watch the time
themselves: the six loops of the ordered locks (`src/sync/ordered.rs`), the
wait for TLB shootdown acknowledgements (`Flusher::flush`) and the wait for
another CPU's shootdown flag (`shootdown_tlb_ipi`). A spin that lasts 2 s
writes one line straight to the PL011: `[spin <cpu> lock <line> <lock
address> 0]`, `[spin <cpu> tlb-ack <line> <acks> <expected>]` or `[spin
<cpu> tlb-flag <line> <target cpu> 0]`. No line after a freeze means the
CPUs spin somewhere else (for example a `spin` crate lock such as the run
queue). It takes the locks with `try_read`,
so a locked list or context shows as `[hbctx ... locked]` instead of a
deadlock.

```bash
scripts/build-image.sh --experiment pi-heartbeat
```

## Results

QEMU raspi3b (`-smp 4`), 2026-10-31, test build `v0.3.0-dev-892-gfaec08e+xccf1`
(`relibc-tests`, `timesync`, `self-exit-status`, `futex-wait-lock`,
`pi-heartbeat`): all 4 CPUs write their heartbeat (`[hb 0 12]` … `[hb 3 12]`),
the relibc suite has 18 failures, all known, none new.
