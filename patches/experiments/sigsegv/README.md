# Experiment: sigsegv, signals for exceptions and sigaltstack on aarch64

relibc registered no exception handler on aarch64, so a bad memory access,
an undefined instruction or a `brk` ended the process without a signal: no
SIGSEGV handler ran, and the kernel printed `FATAL: Not an SVC induced
synchronous exception`. sigaltstack was accepted but never used.

Relibc patch `0001` (branch `exp/sigsegv` in redox-rpi/relibc):

- `__relibc_internal_excpentry`: the exception entry, with the same frame
  as the signal entry; it switches to the sigaltstack if the signal has
  `SA_ONSTACK`, runs `inner_excp` and returns through the signal entry.
- Exception class (ESR_EL1.EC) to signal: aborts SIGSEGV (`SEGV_MAPERR`),
  pc/sp alignment SIGBUS, undefined instruction SIGILL, FP trap SIGFPE,
  `brk` SIGTRAP.
- The signal entry switches to the sigaltstack with `SA_ONSTACK`, and
  `uc_stack` reports it.

Not done: `SEGV_ACCERR` and SIGBUS for alignment faults need the fault
status code, which the kernel does not pass (it passes only the class).
The kernel still prints the register dump for every such exception.

```bash
scripts/relibc-tests.sh --experiment sigsegv
```

## Results
