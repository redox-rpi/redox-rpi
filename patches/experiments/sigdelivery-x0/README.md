# Experiment: sigdelivery-x0, keep the syscall result when a signal arrives

Kernel fix for the start crashes of T12 (and possibly T1) on aarch64.

## The bug

When the kernel sets up a signal handler (`signal_handler` in
`src/context/signal.rs`), it saves the interrupted instruction pointer and
one "arch-dependent" register in the thread's sigcontrol page; the signal
entry in relibc (`__relibc_internal_sigentry`) uses that register as
scratch and restores it from there at the end. On x86_64 that register is
`rflags`, on riscv64 `x5`, on **aarch64 `x0`**, which is also the
register that carries a syscall's return value.

`signal_handler` also runs in the middle of syscalls: in `sched_yield`, and
from `tick()` after a context switch. At that point `x0` still holds the
syscall's **first argument**. The syscall then writes its result into `x0`,
but the signal entry restores the saved value. So on aarch64 a syscall
during which a signal handler is set up returns its first argument instead
of its result.

How T12 showed it: `relibc_start_v1` begins with `relibc_verify_host()`,
which calls `SYS_YIELD` with all arguments `!0` and aborts (`brk #1`) if
the result is an error. With a signal pending at start (from the shell's
job control, for example), the yield "returns" `-1` = `-EPERM`, and the
program dies before `main` with exit code 1. The `brk #1` is shared with
the `expect("no filetable fd present")` next to it, which is why the crash
looked like a missing filetable fd. The register dumps match: `x8` = 158
(`SYS_YIELD`), `x0`..`x4` = `-1`, `x30` still in the program's start code.

## The fix

In the aarch64 syscall path (`src/arch/aarch64/interrupt/exception.rs`),
remember `ELR_EL1` before the syscall. If it changed, a signal handler was
set up during the syscall: store the result in `saved_archdep_reg` as well
(`set_saved_syscall_ret` in `src/context/signal.rs`). ELR is read with
`read_volatile`: `stack` is a `&mut`, and the signal code changes the frame
through `Context::regs_mut`, so the compiler folded a plain comparison
away (the first version of this patch did nothing because of that).

## Build

```bash
scripts/build-image.sh --experiment ldso-diagnostics --experiment timesync --experiment sigdelivery-x0
```

## Result

How the cause was found: experiment `t12-kernel-diag` logged every signal
delivery. Each start of `ps` that crashed got SIGCONT (pending `0x20000`)
from the shell during the `SYS_YIELD` of `relibc_verify_host`, with saved
ip `0x1bb9dc` (after the `svc`) and saved `x0` = `-1`.

QEMU raspi3b, `ps` started 60 times each way, same experiments as v0.2.0
(`ldso-diagnostics`, `timesync`):

| Build | `ps > /dev/null` | `ps` to the console | `ps \| cat` |
|---|---|---|---|
| `v0.2.0-5-gd1f913a+xee90` (first version, check optimized away) | 19 of 60 failed | 35 of 60 | 0 of 60 |
| `v0.2.0-4-g8405d0c+xfc05` (without fix, 40 each) | 15 of 40 | 18 of 40 | 0 of 40 |
| `v0.2.0-9-g43f697e+xe0ce` (this fix) | **0 of 60** | **0 of 60** | 0 of 60 |

Without `ldso-diagnostics` (`v0.2.0-9-g43f697e+x0f50`, relibc `0005`
only), 4 of 300 starts still failed, but differently: data aborts
(`ESR_EL1` `0x92000004`/`44`/`4F`), `ELR` `0xC046FC` twice and `0xC02F78`
in `pwd`, `ls`, `ps`, `sha256sum`. That is T1, a separate bug.

Real Pi 3B+ (test #11, 2026-10-28, [test reports](../../../docs/test-reports.md)):

- `v0.2.0-9-g43f697e+xe0ce` (with `ldso-diagnostics`): `pi-test.py` passes
  `ps-tty` for the first time; `ps` straight to the terminal 80 of 80;
  `reboot` 5 of 5 at the first attempt; starts 0 of 300.
- `v0.2.0-9-g43f697e+x0f50` (without `ldso-diagnostics`): all checks pass;
  1 of 900 starts failed, a data abort (T1), not T12.
