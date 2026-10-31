# Experiment: cputop

Issue [#58](https://github.com/redox-rpi/redox-rpi/issues/58). Shows who
uses the CPU right now, so that whatever keeps a core busy can be found
on any image. The trigger was Pi test #25: the ondemand governor
([#23](https://github.com/redox-rpi/redox-rpi/issues/23)) never saw an
idle Pi.

A package only, no patches, so it goes into any image:

```bash
scripts/build-image.sh --experiment cputop [--experiment …]
```

## Tool `cputop`

```
cputop [-n rounds] [-l lines] [seconds]
```

It reads `/scheme/sys/stat` and `/scheme/sys/context` twice, `seconds`
apart (default 5), and prints:

- per core, the share of the time it was busy (user, nice, kernel and irq
  against idle, in the kernel's milliseconds) and the context switches
  per second;
- the `lines` threads (default 10) that used the most CPU in between: PID,
  CPU % (100 = one core), the total CPU time since the thread started,
  STAT and the core it was last on, and the name. The kernel keeps the
  CPU time per thread (ACTTIME) in hundredths of a second.

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
