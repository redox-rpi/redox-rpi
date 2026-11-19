# Interrupt and timer counters (experiment `irq-stats`)

Issue [#564](https://github.com/redox-rpi/redox-rpi/issues/564). This is a
diagnostic helper, so interrupt and wake-up problems can be seen instead
of guessed. `/scheme/sys/irqstat` shows three tables:

```
kind cpu0 cpu1 cpu2 cpu3        interrupts by kind on each CPU
ipi …                           inter-processor interrupts
timer …                         the per-CPU timer (scheduler tick)
line …                          device lines, handed to their driver
empty …                         controller had nothing pending
unexpected …                    unknown numbers

line cpu0 … max_handler_us      each device line (virtual IRQ number) that fired,
                                its count on each CPU and the longest handler time

wake_late <10us:… <100us:… <1ms:… <10ms:… <100ms:… >=100ms:… max_us:…
                                how late sleeping contexts (nanosleep, timed
                                waits) woke after their wake-up time
```

The counters start at boot. Read the file twice and subtract to get rates.
`/scheme/sys/irq`, upstream's count per user-space IRQ number, stays as it is.

Helps:
- #372 (lost nanosleep wakeup): a wake-up that is late shows in `wake_late`.
  If a sleeper never wakes, the timer kind still counts while `wake_late`
  does not grow.
- The interrupt rates of the SD driver (#307) and dwc2d.

Kernel patch `0001`:
- **aarch64:** counts in the interrupt dispatcher (`interrupt/irq.rs`), with the
  handler time from the generic timer counter.
- **All architectures:** the scheduler's wake check (`context/switch.rs`)
  counts the lateness.

The counters are relaxed atomics, with no locks.

## Test

1. QEMU `raspi3b`: `cat /scheme/sys/irqstat` twice, a few seconds apart.
   The timer ticks on every CPU, the device lines and `wake_late` should
   all grow.
2. Pi 3B+, no person: the same, plus `scripts/pi-test.py`.

## Results

See the PR and the pi-tests rows.
