# Experiment: cputop

Issue [#58](https://github.com/redox-rpi/redox-rpi/issues/58). Shows who
uses the CPU right now, so that whatever keeps a core busy can be found
on any image. The trigger was Pi test #25: the ondemand governor
([#23](https://github.com/redox-rpi/redox-rpi/issues/23)) never saw an
idle Pi.

The tool plus one kernel patch. Without the patch the tool still works,
with the older per-thread times (see below):

```bash
scripts/build-image.sh --experiment cputop [--experiment …]
```

## Tool `cputop`

```
cputop [-n rounds] [-l lines] [-w] [seconds]
```

It reads `/scheme/sys/stat`, `/scheme/sys/context`, `/scheme/sys/cputime`
and `/scheme/sys/irq` twice, `seconds` apart (default 5), and prints:

- per core, the share of the time it was busy (user, nice, kernel and irq
  against idle, in the kernel's milliseconds) and the context switches
  per second;
- the interrupts per second, in total and per interrupt line;
- the `lines` threads (default 10) that used the most CPU in between: PID,
  CPU % (100 = one core), wakeups per second (`WAKE/s`, how often the
  thread was switched to), the total CPU time since the thread started,
  STAT and the core it was last on, and the name. `-w` sorts them by
  wakeups instead of CPU time.

At idle the wakeups matter more than the CPU time: a thread that wakes a
thousand times a second uses almost no CPU time but keeps its core out of
the low-power wait (#126).

## Kernel patch `0001`: `sys:cputime`

`sys:context` gives each thread's CPU time only in hundredths of a second,
and the kernel didn't count how often a thread runs. [`kernel/0001`](kernel/)
adds a switch counter to each context and the file `sys:cputime`: per
thread, in the order of `sys:context`, the PID, the core it was last on,
the CPU time in nanoseconds, the switch count and the name. `ps` and
`sys:context` don't change.

Without the patch `cputop` uses the ACTTIME of `sys:context`, and the
`WAKE/s` column shows `-`.

Time per thread *per core* is not kept by the kernel either (only a total
and the last core). The per-core busy lines and each thread's last core
were enough to find the busy threads so far, so it is left out.

In QEMU, 20 s after boot (main, `v0.3.0-dev-317-g40be428+xe828`):

```
cputop: 10.0 s, 33 threads
cpu0  busy   0 %  (23 ms busy, 9703 ms idle, 101 switches/s)
cpu1  busy   0 %  (2 ms busy, 9364 ms idle, 242 switches/s)
cpu2  busy   0 %  (20 ms busy, 9410 ms idle, 173 switches/s)
cpu3  busy   0 %  (0 ms busy, 9466 ms idle, 151 switches/s)
   PID   CPU%        TOTAL STAT  CPU  NAME
    13    1.2    0:00:01.0 UB    #1   /scheme/initfs/bin/dwc2d
    18    0.2    0:00:00.3 UB    #2   /usr/bin/ptyd
    17    0.1    0:00:01.9 UB    #0   /scheme/initfs/bin/redoxfs
   …
```

Without kernel patch `cpu_stats` `0008` (experiment `cpu-ondemand`), the
per-core lines miss the time of short context switches (rounded down to
whole milliseconds), so a core can look idler than it is. The per-thread
times are not affected.

## QEMU

[`qemu-check.sh`](qemu-check.sh): boots, waits 20 s, runs `cputop 10` and
a few option checks, and prints the idle table.

QEMU check (`v0.3.0-dev-317-g40be428+xe828`): 6 of 6 pass. Idle after
boot, no thread stays busy: `dwc2d` 1.2 %, everything else under 0.3 %.
After a loop of 400 `ls /` runs, the initfs `redoxfs` had 1 min 22 s of
CPU time (about 0.2 s per program start), but at idle it is back to 0 %,
in the first 10 s and 30 s later. So in QEMU nothing keeps a core busy
after load. In Pi test #25 the 14 min of `redoxfs` fit the 6500-start
soak; what kept the Pi busy after it is for Pi test #82 to find.

## QEMU with `sys:cputime`

[`qemu-check.sh`](qemu-check.sh) now also checks `sys:cputime`, the
`WAKE/s` column, the interrupt line and `-w`: 8 of 8 pass
(`v0.4.0-dev-393-g5dcdf6a+x148e`). Idle, 20 s after boot:

```
cputop: 10.0 s, 39 threads
cpu0  busy   0 %  (0 ms busy, 10012 ms idle, 99 switches/s)
cpu1  busy   0 %  (0 ms busy, 10012 ms idle, 162 switches/s)
cpu2  busy   0 %  (33 ms busy, 9989 ms idle, 181 switches/s)
cpu3  busy   1 %  (184 ms busy, 9828 ms idle, 278 switches/s)
irq   100/s  (3: 100)
   PID   CPU%  WAKE/s        TOTAL STAT  CPU  NAME
    16    0.8      58    0:00:00.6 UB    #3   /scheme/initfs/bin/dwc2d
     8    0.5       9    0:00:00.3 US    #3   /scheme/initfs/bin/rpi-firmwared
    25    0.2       9    0:00:00.1 UB    #3   /usr/bin/netstack
    21    0.1      10    0:00:00.2 UB    #2   /usr/bin/ptyd
   …
     0    0.0      58    0:00:00.0 RR+   #3   [kmain]
```

- `[kmain]` rows are the cores' idle threads. Their `WAKE/s` is how often
  that core went idle, and their CPU time stays 0 because idle time isn't
  counted as CPU time.
- In QEMU the only interrupt is the 100 Hz timer (line 3).
- `dwc2d` is the busiest thread, with 58 wakeups a second (its USB polling).
