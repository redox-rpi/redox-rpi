# Experiment: sigregs, register test for the signal entry

Adds [`tools/sigregs`](../../../tools/sigregs) to the image. It checks that
taking a signal leaves every register of the interrupted code unchanged:
the main thread loads a known pattern into x1-x17, x19-x29 and v0-v31 and
checks them in a loop while signals arrive, and the handler overwrites all
of them. A register that differs afterwards shows a bug in relibc's
`__relibc_internal_sigentry`, like the x1/x2 swap that relibc patch `0004`
fixes.

```bash
scripts/build-image.sh --experiment sigregs-test
```

On the target:

```text
sigregs                 # both modes, 1000 signals each
sigregs thread -n 5000  # one mode, more signals
```

- `process`: a child process sends `SIGUSR1` with `kill()`.
- `thread`: another thread sends it with `pthread_kill()`.

They take different paths through the signal entry. Each mode prints
`PASS` or `FAIL` with the register that changed, or how many signals
arrived before they stopped (for example because a signal stayed blocked).
Exit code 0 when every mode passed.

Not checked: x18 (the signal entry may use it), x30 (scratch register of
the loop), x0, `nzcv` and `sp`. The handler cannot overwrite x19 (LLVM
reserves it for inline asm).

The test also runs on macOS and Linux on aarch64 (`cargo run` in
`tools/sigregs`). On macOS it passes, and it reports `x1` when the handler
swaps x1 and x2 in the saved context, and the right half of `v5` when it
flips a bit there.

## Results

See [test reports](../../../docs/test-reports.md).
