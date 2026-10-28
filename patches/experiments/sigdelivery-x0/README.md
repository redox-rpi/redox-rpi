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
(`set_saved_syscall_ret` in `src/context/signal.rs`).

## Build

```bash
scripts/build-image.sh --experiment ldso-diagnostics --experiment timesync --experiment t12-auxv-dump --experiment sigdelivery-x0
```

## Result

Not run yet.
