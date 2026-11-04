# Experiment: sigsegv, signals for exceptions and sigaltstack on aarch64

relibc registered no exception handler on aarch64, so a bad memory access,
an undefined instruction or a `brk` ended the process without a signal: no
SIGSEGV handler ran, and the kernel printed `FATAL: Not an SVC induced
synchronous exception`. sigaltstack was accepted but never used.

Default relibc patch [`relibc/0014`](../../relibc/) now (`0001` here,
branch `exp/sigsegv` in redox-rpi/relibc), applied after `0008`, `0009`
and `0013` without this experiment; this README keeps the history and the
name is still accepted. Numbered `0010` before 2026-10-31; ported onto
`0013` then (the exception entry returns through `svc #0x5352` like the
signal entry):

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
scripts/relibc-tests.sh
```

## Results

QEMU raspi3b, `scripts/relibc-tests.sh` (2026-10-29):

| Build | Experiments | Failures | Fixed |
|---|---|---|---|
| `v0.3.0-dev-218-gb8567d6+x2fad` | relibc-tests (the list) | 25 | |
| `v0.3.0-dev-221-g6f7c5c6+x3e10` | + sigsegv | 23 | `signals/sigsegv`, `signals/stackoverflow` |
| `v0.3.0-dev-225-gd069268+x7e8c` | + sigsegv + sigentry-thread-signals (local merge with `t4-thread-signals`) | 17 | the two above, `sigaltstack`, and the five #4 tests |

`sigaltstack` needs both: it `raise`s SIGUSR2 from its own handler, a
signal sent to the thread, which is only delivered with
`sigentry-thread-signals` ([#4](https://github.com/redox-rpi/redox-rpi/issues/4)).
No new failures. The patches apply together.

Real Pi 3B+, 2026-10-29: test 68 (with `sigentry-thread-signals`) passes:
`signals/sigsegv`, `signals/stackoverflow` and `sigaltstack` pass, no new
failure ([test reports](../../../docs/test-reports.md)).
