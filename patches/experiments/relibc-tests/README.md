# Experiment: relibc-tests, relibc test suite on Redox

Adds upstream's `relibc-tests-bins` package: relibc's `tests/`, built for
Redox with our relibc patches, in `/usr/share/relibc-tests`
([#13](https://github.com/redox-rpi/redox-rpi/issues/13)).

```bash
scripts/relibc-tests.sh            # build, run in QEMU, compare
scripts/build-image.sh --experiment relibc-tests   # only the image
```

The image gets `/usr/share/relibc-tests/run.ion`, which runs the suite
like upstream's `make run` without needing `make`:

```text
ion /usr/share/relibc-tests/run.ion    # ends with RELIBC-TESTS rc=0 or 1
```

## Automatic runs

[`scripts/relibc-tests.sh`](../../../scripts/relibc-tests.sh) builds the
network-boot image with this experiment (plus any `--experiment` given),
boots it in QEMU's raspi3b machine, runs the suite and compares the
failures with
[`config/relibc-tests-expected.txt`](../../../config/relibc-tests-expected.txt)
([`relibc-tests-compare.py`](../../../scripts/relibc-tests-compare.py)):

- `NEW <test>`: fails and is not on the list. Exit status 1.
- `FIXED <test>`: on the list as `fail`, but passed. Remove it from the
  list; the run still passes.
- Tests marked `flaky` may pass or fail.
- Exit status 2: the build failed or the suite did not finish.

The result goes to `build/relibc-tests/<build ID>.txt`, the serial log
next to it. A run takes about 12 minutes (8 build, 4 QEMU). It needs no Pi.

Nightly: [`config/redox-relibc-tests.service`](../../../config/redox-relibc-tests.service)
and `.timer` run it at 01:30 UTC on `origin/main` in
`~/redox-rpi-relibc-tests`, with `--issue`: a result that differs from the
list opens or comments on the issue "Nightly: relibc tests differ from the
expected failures". Not installed yet, and it needs this experiment on
`main`. GitHub's runners cannot do it: they build only the kernel (PR #41);
the full image needs the build host's toolchain and cookbook.

## Static programs and the toolchain's libc.a

Upstream's recipe builds the tests with `NATIVE_LIBC=1`, so gcc links the
static tests with the `libc.a` in the toolchain's sysroot
(`prefix/aarch64-unknown-redox/sysroot`). That copy is made once, when the
toolchain is built (`mk/prefix.mk`: the relibc recipe's stage is copied in),
here on 2026-10-24, and never follows our relibc patches. The same holds
for every static C program a recipe builds without depending on relibc.

This experiment replaces the recipe
([`recipes/tests/relibc-tests-bins/recipe.toml`](recipes/tests/relibc-tests-bins/recipe.toml);
`build-image.sh` puts an experiment's `recipes/<path>/recipe.toml` in place
of upstream's and builds it from scratch): relibc is a build dependency, so
its stage is in the recipe's sysroot, and gcc gets
`-B<sysroot>/lib/ -L<sysroot>/lib`, so it finds `libc.a`, `libc.so` and
`crt*.o` there before the toolchain's. Checked with `-Wl,--trace`, and in
`bins_static/args`: 3714 strings that only our `libc.a` has, 2 that only
the toolchain's has. The headers still come from the toolchain (putting
relibc's first breaks `#include_next` in libstdc++).

Other ways, for later: rebuild the prefix sysroot when relibc changes
(upstream's `make prefix` after `static_clean`, slow, and it changes the
toolchain for all builds), or the same build dependency for other static
recipes (`sigregs` already does this).

## Relibc patches (in the tests only)

- `0001` `tls_align`: thread-local data aligned to 128.
- `0002` `tls_shared`: TLS of a shared library loaded at start
  (TLSDESC and initial exec).
- `0003` `tls_probe`: prints the TLS layout relative to the thread
  pointer (no pass/fail).

Limits: no shell (`sh`) in the image, no network in QEMU.

## Results

The verifier counts as failures: wrong output or exit status, tests that
hang (10 s), and status-only tests (`-s`) that exit non-zero. The first
result here (`v0.3.0-dev-147-g16cb3e8+x2fad`, "15 fail") missed the last
two kinds; that run had 28 failures.

QEMU, `v0.3.0-dev-218-gb8567d6+x2fad` (main + this experiment, static tests
with our `libc.a`), three runs: 319 tests, 25 fail every time,
`killpg0-self` once. The list with causes is
[`config/relibc-tests-expected.txt`](../../../config/relibc-tests-expected.txt).
Compared with the first run, `bins_static/destructor` and
`bins_static/pthread/init` no longer hang.

With the fixes for #4 and #14 (`--experiment tls-variant1 --experiment
sigentry-thread-signals`, `v0.3.0-dev-229-g5a08d6c+xca65`, a local merge of
the branches `t14-tls-layout` and `t4-thread-signals`): 16 fail, none new, 9
fixed: `sigaction`, `pselect`, `sigpause-revert`, `sigprocmask-3`,
`sigset-5`, `stdio/mutex`, `tls_shared`, dynamic and **static**
`tls_align`. Static `tls_align` failed with the toolchain's `libc.a` even
with `tls-variant1`; it passes now because it links this build's relibc.
