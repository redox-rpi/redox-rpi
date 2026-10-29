# Experiment: relibc-tests, relibc test suite on Redox

Adds upstream's `relibc-tests-bins` package: relibc's `tests/`, built for
Redox with our relibc patches, in `/usr/share/relibc-tests`
([#13](https://github.com/redox-rpi/redox-rpi/issues/13)).

```bash
scripts/build-image.sh --experiment relibc-tests
```

The image gets `/usr/share/relibc-tests/run.ion`, which runs the suite
like upstream's `make run` without needing `make`:

```text
ion /usr/share/relibc-tests/run.ion    # ends with RELIBC-TESTS rc=0 or 1
```

Relibc patches (in the tests only):

- `0001` `tls_align`: thread-local data aligned to 128.
- `0002` `tls_shared`: TLS of a shared library loaded at start
  (TLSDESC and initial exec).
- `0003` `tls_probe`: prints the TLS layout relative to the thread
  pointer (no pass/fail).

Limits:

- **The static tests link the toolchain's `libc.a`**
  (`prefix/.../sysroot`), not the relibc of this build: the recipe builds
  with `NATIVE_LIBC=1`. They test whatever relibc the toolchain was built
  with. The dynamic tests use the image's `libc.so`.
- No shell (`sh`) in the image, no network in QEMU.

## Results

QEMU, 2026-10-29, `v0.3.0-dev-147-g16cb3e8+x2fad` (main + this
experiment): 395 test runs, 15 fail.

| Test | Result | Cause |
|---|---|---|
| `tls_align` (static, dynamic) | initial values 0 | aarch64 TLS layout in `ld.so`, [#14](https://github.com/redox-rpi/redox-rpi/issues/14), experiment `tls-variant1` |
| `sigaction` | second handler not called after `raise` | thread signal blocks all signals, [#4](https://github.com/redox-rpi/redox-rpi/issues/4) (to check with `sigentry-thread-signals`) |
| `sys_select/pselect` | SIGILL | signal path, to check |
| `sigaltstack` | panic (`brk`) | sigaltstack not implemented on aarch64 (`arch_pre` is a TODO) |
| `stdio/mutex` | `ftrylockfile` succeeds in another thread | to check; passed in the next run |
| `time/timer` | `sem_wait` EINVAL | to check |
| `stdio/fscanf_offby1`, `stdio/ungetc_ftell` | off by one | to check (also on x86_64?) |
| `stdio/popen`, `stdlib/system`, `unistd/exec` | no shell or program not found | image |
| `sys_socket/getsockopt`, `recv`, `recvfrom` | `socket` ENODEV | no network in QEMU |
