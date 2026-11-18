# Experiment: tcc, TinyCC as a native C compiler

For [#247](https://github.com/redox-rpi/redox-rpi/issues/247), topic
[tcc](../../../docs/topics/tcc.md). Builds TinyCC from the `mob` branch
(arm64 backend, 70adbb12, 2026-11-07) for aarch64 Redox and puts it into
the image with what it needs to compile on the Pi:

- `recipes/wip/dev/lang/tcc/recipe.toml` replaces upstream's stub recipe
  (tcc 0.9.27 tarball, no cross flags). tcc is cross-compiled with the
  Redox gcc (`--cpu=arm64 --targetos=Linux`, ELF like Linux) and runs on
  Redox; its search paths are `/usr/lib/tcc/include:/usr/include`,
  `/usr/lib/tcc:/usr/lib`, crt files in `/usr/lib`, ELF interpreter
  `/lib/ld.so.1` (what Redox' dynamic binaries use).
- relibc's headers, `crt0.o`/`crt1.o`/`crti.o`/`crtn.o`, `libc.so` and
  `ld.so.1` are already in the image: the `relibc` package installs its
  whole stage.
- `tools/tcc-bench` (package `tcc-bench`): test sources in
  `/usr/share/tcc-tests/`, the benchmark built by gcc -O2 (`bench-gcc`) for
  comparison, `timeit` (wall time of a command; Ion has no `time`), and
  `run.ion`, which runs all checks.
- The patched tcc sources with their `config.h` are in `/usr/src/tinycc`
  for the self-hosting test.

Run the checks: `ion /usr/share/tcc-tests/run.ion`.

## x18

relibc's aarch64 signal entry returns with `br x18` (relibc `0007`, #1), so
no code may keep a value in x18. tcc's arm64 backend never allocates x18:
`reg_classes` in `arm64-gen.c` leaves x18 out of `RC_INT` ("reserved on
Windows/OSX"), and the only explicit use is Windows TLS. No backend patch
is needed. The tcc binary and `libtcc1.a` are built by gcc with
`-ffixed-x18` (the Redox gcc does not reserve it by default); `objdump`
finds x18 only in libgcc's `__multf3` and `__divtf3` (long double soft
float, linked into tcc from the toolchain, which is not built with
`-ffixed-x18`; that affects every gcc-built program, not only tcc).

## Patches to tinycc

The recipe lists them under their own names; `build-image.sh` copies a
whole experiment recipe's patches next to it.

- `0001` the Redox tcc (built with `-DTARGETOS_Redox=1`) defines
  `__redox__`, not `__linux__`. relibc's headers choose Redox values on
  `__redox__` (open flags, `suseconds_t`) and Linux values on `__linux__`.
- `0002` declare `environ` in `tccrun.c`: relibc declares it in no header.
- `0003` `__clear_cache` for arm64 in `lib/armflush.c` when gcc builds
  `libtcc1.a`: `__arm64_clear_cache` is a tcc builtin and stayed an
  undefined symbol.
- `0004` gcc's integer type macros (`__INT_LEAST8_TYPE__`, `__INTPTR_MAX__`,
  `__INT8_C`, ...) for Redox aarch64: relibc's `stdint.h` is gcc's
  `stdint-gcc.h` and needs them. `wchar_t` is `unsigned int` as with gcc.
  `c2str` maps `__redox__` to `TARGETOS_Redox`, since it evaluates the
  `#if` lines of `tccdefs.h` when tcc is built.

## Build notes

- The image build's make variables (`ARCH=aarch64`) reach tinycc's make
  through `MAKEFLAGS` and replace its `ARCH=arm64`: no arm64 sources and an
  empty `libtcc.a`. The recipe unsets `MAKEFLAGS`.
- `c2str.exe` (turns `include/tccdefs.h` into `tccdefs_.h`) runs on the
  build host, so the recipe builds it with the host `cc` first.
- `libtcc1.a` is built by the cross gcc (`arm64-libtcc1-usegcc=yes`), since
  the new tcc cannot run on the build host.
- `scripts/build-image.sh` now takes experiment recipes at any depth
  (`recipes/**/recipe.toml`), for `wip/dev/lang/tcc`.

## Results

QEMU, `v0.4.0-dev-689-gb453b397+xd12c`: dynamic Hello World, `tcc -run`,
the benchmark and self-hosting (tcc compiles `tcc.c` into a working
`tcc2`) pass; `-static` fails on relibc's TLS descriptor relocations.
In the release image from v0.4.1 as a beta
([#293](https://github.com/redox-rpi/redox-rpi/issues/293)); sizes in the
topic page.
Real Pi 3B+ ([test #278](https://github.com/redox-rpi/pi-tests/blob/main/reports/278.md)):
the same checks pass; Hello World compiles in 100 ms, `tcc.c` (77k lines) in
0.7 s; tcc's code is 1.3-3.3 times slower than gcc -O2's. Numbers in the
[topic page](../../../docs/topics/tcc.md).
