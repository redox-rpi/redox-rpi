# Experiment: t12-kernel-diag, kernel log for signal delivery during syscalls

Diagnostics for T12, on top of the first version of `sigdelivery-x0`
(build `v0.2.0-8-gfa0a4d4`; it does not apply to the current one).
Logs at error level, so they reach the serial console:

- `t12diag: SYS_YIELD returned …`: a yield that did not return 0;
- `t12diag: handler set up during syscall …`: ELR changed during a syscall;
- `t12diag: deliver, pending …`: every signal delivery, with the pending
  unblocked signals and whether it happens inside a syscall.

```bash
scripts/build-image.sh --experiment ldso-diagnostics --experiment timesync --experiment t12-auxv-dump --experiment sigdelivery-x0 --experiment t12-kernel-diag
```

## Result

QEMU, `ps` straight to the console 40 times (build `v0.2.0-8-gfa0a4d4+x5010`):
every start that got a signal (SIGCONT from the shell) during the
`SYS_YIELD` of `relibc_verify_host` logged `saved ip 0x1bb9dc x0 -1`: the
address after the `svc` and the first argument of the yield. Without the
fix of `sigdelivery-x0`, that `x0` comes back as the result of the yield,
which is the T12 crash.

It also showed why the first version of `sigdelivery-x0` did nothing: the
compiler folded its ELR check away (`stack` is a `&mut`, and the signal
code changes the frame through another pointer). The logging here lets the
pointer escape, so the check worked: 0 of 40 failed.
