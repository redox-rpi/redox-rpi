# Experiment: t12-kernel-diag, kernel log for signal delivery during syscalls

Diagnostics for T12, on top of `sigdelivery-x0` (apply that one first).
Logs at error level, so they reach the serial console:

- `t12diag: SYS_YIELD returned …`: a yield that did not return 0;
- `t12diag: handler set up during syscall …`: ELR changed during a syscall;
- `t12diag: deliver, pending …`: every signal delivery, with the pending
  unblocked signals and whether it happens inside a syscall.

```bash
scripts/build-image.sh --experiment ldso-diagnostics --experiment timesync --experiment t12-auxv-dump --experiment sigdelivery-x0 --experiment t12-kernel-diag
```

## Result

Not run yet.
