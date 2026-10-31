# Experiment: pi-heartbeat, does the kernel still run?

Diagnostic kernel patch for the whole-machine freeze in
[#14](https://github.com/redox-rpi/redox-rpi/issues/14) (`pthread/rwlock_randtest`
on the real Pi, tests 166, 170, 173). Not meant for main.

Every 1000 timer interrupts (10 s at 100 Hz) each CPU writes
`[hb <cpu> <n>]` to the serial console, straight to the PL011 data
register and without the COM1 lock (`uart_pl011::raw_write`). Output can
interleave with other console output.

What a freeze then shows in the serial log:
- all CPUs keep writing: the CPUs take timer interrupts; the freeze is
  in the scheduler or a lock with interrupts on (a deadlock), or in user
  space / devsh.
- some CPUs stop: those CPUs hang with interrupts off (a spin lock held
  in interrupt context, or a fault loop).
- all stop at once: the whole machine (interrupt controller, bus, memory)
  or every CPU in the same lock.

```bash
scripts/build-image.sh --experiment pi-heartbeat
```

## Results

QEMU raspi3b (`-smp 4`), 2026-10-31, test build `v0.3.0-dev-892-gfaec08e+xccf1`
(`relibc-tests`, `timesync`, `self-exit-status`, `futex-wait-lock`,
`pi-heartbeat`): all 4 CPUs write their heartbeat (`[hb 0 12]` … `[hb 3 12]`),
the relibc suite has 18 failures, all known, none new.
