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
  against idle, in the kernel's milliseconds);
- the `lines` threads (default 10) that used the most CPU in between: PID,
  CPU % (100 = one core), the total CPU time since the thread started,
  STAT and the core it was last on, and the name. The kernel keeps the
  CPU time per thread (ACTTIME) in hundredths of a second.

```
cputop: 10.0 s, 61 threads
cpu0  busy  12 %  (1210 ms busy, 8790 ms idle)
cpu1  busy 100 %  (10004 ms busy, 0 ms idle)
   PID   CPU%        TOTAL STAT  CPU  NAME
    20   99.8    0:14:11.0 UR    1    /scheme/initfs/bin/redoxfs
   …
```

Without kernel patch `cpu_stats` `0008` (experiment `cpu-ondemand`), the
per-core lines miss the time of short context switches (rounded down to
whole milliseconds), so a core can look idler than it is. The per-thread
times are not affected.

## QEMU

[`qemu-check.sh`](qemu-check.sh): boots, waits 20 s, runs `cputop 10` and
a few option checks, and prints the idle table.
