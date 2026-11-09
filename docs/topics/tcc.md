# Native C compiler (TinyCC)

The first compiler that runs on Redox on the Pi itself: TinyCC (`tcc`),
about 390 KB, compiles and links in one step, `tcc -run` runs a C file
without writing a binary. Chosen in the Architect session as the first
compiler to try. Issue:
[#247](https://github.com/redox-rpi/redox-rpi/issues/247).

## State

Experiment [`tcc`](../../patches/experiments/tcc/README.md) on `main`
(PR [#249](https://github.com/redox-rpi/redox-rpi/pull/249)): tinycc `mob`
(arm64 backend) with four small patches, cross-compiled for aarch64 Redox,
plus test programs (`tools/tcc-bench`).

**In the release image from v0.4.1, as a beta**
([#293](https://github.com/redox-rpi/redox-rpi/issues/293), the
maintainer's decision): the release options get `--experiment tcc` for the
SD card and netboot images and the Zero 2 W image. All three still fit
(build `v0.4.0-dev-819-gab8fedb5`, against v0.4.0-rc2; the tcc package is
3.8 MB, 2.6 MB of it the sources in `/usr/src/tinycc` for self-hosting):

| Image | rc2 | with tcc | RedoxFS partition |
|---|---|---|---|
| SD card, installed | 203 MB | 207 MB | 1 GB |
| SD card `.img.xz` (`xz -6`) | 48.8 MB | 49.7 MB | |
| Netboot (release options) | 206 MB | 210 MB | 232 MB |
| Zero 2 W netboot (`timesync tcc`) | 113 MB | 116 MB | 124 MB |

QEMU, 2026-11-09: on the netboot and Zero 2 W images `tcc -run`, a dynamic
build and the script `hallo.c` work, the desktop's motd is still there.
Real Pi 3B+ ([test
#293](https://github.com/redox-rpi/pi-tests/blob/main/reports/293.md),
release-options netboot image `v0.4.0-dev-819-gab8fedb5+xc80e`): pass.
`tcc -run hello.c` 120 ms, `tcc -o hello` 99 ms, the script `hallo.c`
rc 42, the whole tcc check to its end (static Hello World fails as known),
motd and `EXPERIMENTS` as in the release, `pi-test.py` 6 of 6, 0 of 300
starts failed, no panic.

Line for the release notes:

> **A C compiler on the Pi (beta).** TinyCC (`tcc`) compiles C on the Pi
> itself: `tcc -o hello hello.c` builds a program in about 0.1 s,
> `tcc -run hello.c` runs a file without writing a binary, and a C file
> that starts with `#!/usr/bin/tcc -run` runs as a script. Examples in
> `/usr/share/tcc-tests/`. Known limits: programs are linked dynamically
> only (`-static` fails: relibc's `libc.a` uses TLS descriptor relocations
> that tcc's linker does not know); tcc's code runs 1.3 to 3.3 times slower
> than gcc -O2's; quotes in shebang options are removed. (#293)

## Pi results

**Real Pi 3B+**
([test #278](https://github.com/redox-rpi/pi-tests/blob/main/reports/278.md),
`v0.4.0-dev-689-gb453b397+xd12c`, 2026-11-07): pass, no panic.

| Check | Result |
|---|---|
| Hello World, dynamic (`tcc -o hello hello.c`) | compile 100 ms, run 30 ms |
| `tcc -run hello.c` | 130 ms |
| Hello World, static (`tcc -static`) | **fails** after 47 s: relibc's `libc.a` uses TLS descriptor relocations (`R_AARCH64_TLSDESC_*`, 562-569), which tcc's linker does not know |
| Benchmark compiled by tcc (4616 lines with headers) | compile 162 ms (0.084 s in the compiler, 54,952 lines/s) |
| Self-hosting: tcc compiles `tcc.c` (77,143 lines with headers) | 0.70 s in the compiler (109,733 lines/s), 823 ms wall, warm 831 ms; the new `tcc2` builds Hello World in 101 ms |
| `free` after the tests | 160 of 740 MB used |

Speed of the compiled code, the same benchmark (two runs each, the same
results):

| Part | tcc | gcc -O2 (cross) | tcc / gcc |
|---|---|---|---|
| fib(32), calls | 120 ms | 36 ms | 3.3× |
| sieve 5 × 2M, memory loops | 1245 ms | 950 ms | 1.3× |
| mandelbrot 200, floating point | 115-118 ms | 35 ms | 3.3× |
| total | 1514-1520 ms | 1054-1059 ms | 1.4× |

tcc is a quick edit-compile-run compiler on the Pi (a 77k-line program in
under a second); its code is 1.3-3.3 times slower than gcc -O2's.

QEMU (same image) passed the same checks first; its times (Hello World
compile 230 ms, `tcc.c` 5.5 s) are CPU emulation and say little about the
Pi.

**C files as scripts:** a C file that starts with `#!/usr/bin/tcc -run`,
made executable with `chmod +x`, runs directly (`./hallo.c a b`), from Ion
and through `execvp` (`timeit`). It gets its arguments (`argv[0]` is the
path it was started with) and stdin, and its exit code is the program's.
Options after `-run` in the shebang reach tcc (`-run -DANSWER=42` works),
but quotes in them are removed: `-DX="text"` defines `X` as `text`, not as a
string. No wrapper is needed. Real Pi 3B+ ([test
#282](https://github.com/redox-rpi/pi-tests/blob/main/reports/282.md),
`v0.4.0-dev-716-gc6d4de06+xd12c`): pass, 128 ms per start (warm 127 ms);
QEMU 244 ms. Example: `/usr/share/tcc-tests/hallo.c`.

**x18:** tcc never allocates x18 (`arm64-gen.c` keeps it out of `RC_INT`),
so its code is safe with relibc's signal entry, which clobbers x18
([#127](https://github.com/redox-rpi/redox-rpi/issues/127)). tcc and
`libtcc1.a` are built with `-ffixed-x18`. The tcc binary still has x18 in
libgcc's `__multf3`/`__divtf3`, which come from the toolchain like in every
gcc-built program.

**What was missing in relibc** (worked around in tcc, not changed in relibc):

- `environ` is declared in no header (glibc: `unistd.h` with `_GNU_SOURCE`).
- `stdint.h` is gcc's `stdint-gcc.h`, which only works with gcc's
  predefined type macros; a compiler other than gcc has to define them.
- `libc.a` needs TLS descriptors, so tcc can only link dynamically.
- Warning only: `bits/null.h` redefines `NULL` after tcc's `stddef.h`.

## Next

- Static linking: TLSDESC support in tcc's arm64 linker, or a relibc
  `libc.a` built with traditional TLS (`-mtls-dialect=trad`).
- Ion has no `time`; `timeit` (in `tools/tcc-bench`) fills the gap for now.

## Later

- A second compiler that optimises (gcc or clang for Redox aarch64), for
  programs where tcc's code is too slow.
- tcc in Redox' package repository (upstream's `wip/dev/lang/tcc` recipe is
  a stub). Upstream posts are the user's; sessions draft the text.
