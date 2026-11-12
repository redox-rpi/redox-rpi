# Native C compilers (TinyCC, later QBE)

The first compiler that runs on Redox on the Pi itself: TinyCC (`tcc`),
about 390 KB, compiles and links in one step, `tcc -run` runs a C file
without writing a binary. Chosen in the Architect session as the first
compiler to try (issue
[#247](https://github.com/redox-rpi/redox-rpi/issues/247), done). A second,
optimising small compiler, cproc + QBE, is discussed below under
[Other compilers](#other-compilers-cproc--qbe); nothing is started there.

## State

Experiment [`tcc`](../../patches/experiments/tcc/README.md) on `main`
(PR [#249](https://github.com/redox-rpi/redox-rpi/pull/249)): tinycc `mob`
(arm64 backend) with four small patches, cross-compiled for aarch64 Redox,
plus test programs (`tools/tcc-bench`).

**In the release image from v0.5.0, as a beta**
([#293](https://github.com/redox-rpi/redox-rpi/issues/293), done; the
maintainer's decision; PR [#297](https://github.com/redox-rpi/redox-rpi/pull/297)
merged for v0.5.0, which was called v0.4.1 until 2026-11-09): the release options get `--experiment tcc` for the
SD card and netboot images and the Zero 2 W image. v0.4.0 did not have it.
All three still fit
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
#293](https://github.com/redox-rpi/pi-tests/blob/main/reports/293.md)
(the test name still says v0.4.1),
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

## Other compilers: cproc + QBE

From the Architect discussions with the maintainer (2026-11-04 and
2026-11-07). **Experiment [`cproc-qbe`](../../patches/experiments/cproc-qbe/README.md)
([#327](https://github.com/redox-rpi/redox-rpi/issues/327)) runs on the
Pi**, see [Pi results](#cproc--qbe-on-the-pi) below; not in any image yet,
that needs the maintainer's decision.

[QBE](https://c9x.me/compile/) ([git](https://c9x.me/git/qbe.git/)) is a
small compiler backend, about 10,000 lines, with an arm64 backend (also
amd64 and riscv64; no 32-bit ARM). For C it is used with the front end
[cproc](https://sr.ht/~mcf/cproc/) (C11). Its goal is about 70 % of the
speed of a big optimising compiler with about 10 % of the code.

| | TinyCC | cproc + QBE | gcc / clang |
|---|---|---|---|
| Size | ~390 KB, one program | ~20,000 lines, several programs | 100 MB and more |
| Assembler and linker | built in, writes ELF | none: needs `as` and `ld` (binutils) | binutils / lld |
| Optimisation | almost none, compiles line by line, keeps much on the stack | SSA, register allocation, simple folding | full: inlining, loops, NEON vectorising, Cortex-A53 scheduling |
| Code speed | 1.3-3.3× slower than gcc -O2 (Pi, above) | 1.0-2.3× slower than gcc -O2 (Pi, below) | reference |
| On the Pi today | yes, beta in the image | experiment, Pi-tested | no (upstream has a `dev/gcc13` recipe, not tried for aarch64) |

### cproc + QBE on the Pi

Experiment `cproc-qbe` (build after `tcc`): qbe and cproc cross-built,
GNU `as` and `ld` from Redox' binutils 2.43.1 fork, and `tcc -E -undef`
(tcc patch `0005`) as the preprocessor, since cproc's own is not finished.
The chain adds 4.2 MB on top of tcc (`as` 2.1 MB, `ld` 1.6 MB, qbe and
cproc 0.35 MB). Since #327 step 2 the experiment also has `ar`, `ranlib`
(a script running `ar s`), `nm`, `objdump` and `strip`, another 5.5 MB:
static libraries can be made and linked on the Pi (QEMU-tested; Pi row in
the PR).

Real Pi 3B+ ([test 317](https://github.com/redox-rpi/pi-tests/blob/main/reports/317.md),
`v0.5.0-dev-25-gd3ca14bc+x898d`, 2026-11-10): pass, no panic, `pi-test.py`
6 of 6.

| Check | cproc + QBE | tcc |
|---|---|---|
| Hello World, dynamic | compile 422 ms, run 31 ms | compile 100 ms |
| Hello World, **static** | compile 16.8 s, run 28 ms: **works**, GNU ld links relibc's TLS descriptor relocations | fails |
| Steps alone (`bench.c`) | `tcc -E -undef` 113 ms, `cproc -S` 232 ms, `as` 68 ms | |
| Benchmark compile | 480 ms | 152 ms |

Speed of the compiled code, the same benchmark as above:

| Part | cproc | tcc | gcc -O2 | cproc / gcc |
|---|---|---|---|---|
| fib(32), calls | 82.4 ms | 118.3 ms | 36.1 ms | 2.3× |
| sieve 5 × 2M, memory loops | 914.7 ms | 1146.4 ms | 865.8 ms | 1.06× |
| mandelbrot 200, floating point | 52.8 ms | 115.3 ms | 35.3 ms | 1.5× |
| total | 1052 ms | 1381 ms | 938 ms | 1.12× |

cproc's code is clearly faster than tcc's (fib 1.4×, mandelbrot 2.2×) and
on the memory-bound sieve close to gcc -O2; it compiles 3 times slower
than tcc, still under half a second for the benchmark. Static programs
are possible with it, not with tcc.

Found on the way (relibc, worked around, to fix there):

- `fscanf` loses the character it looks ahead at on a pipe (qbe read
  cproc's `data` as `dta`); qbe patch `0002` avoids `fscanf`.
- `signal.h` has an empty member declaration in `struct ucontext` on
  aarch64; cproc patch `0002` accepts it.
- `as` and `ld` print `TODO: getrusage … not implemented` once.

What it took on Redox aarch64 (the plan before the experiment):

- Build `qbe` and `cproc` for `aarch64-unknown-redox`. Upstream Redox has a
  WIP recipe for QBE without a build script.
- An assembler and linker on the Pi: binutils for Redox aarch64 (upstream
  has a package, not checked for aarch64), or a smaller assembler/linker.
- The same relibc gaps as tcc (`stdint.h` from gcc, `environ`, TLS
  descriptors in `libc.a`), see above.

Where QBE fits:

- **On the Pi:** a step up from tcc for programs where tcc's code is too
  slow but gcc or clang are too big. It does not use NEON or A53
  scheduling either; code that has to be fast is still built with
  rustc/LLVM on the build host.
- **For the open firmware:** a VideoCore IV backend for QBE, with vasm as
  the assembler, could one day replace the old VC4 GCC toolchain. See
  [open-firmware.md, VPU compiler](open-firmware.md). TCC does not suit
  that, because it would need its own VC4 assembler and linker.

## Later

- A second compiler that optimises, for programs where tcc's code is too
  slow: cproc + QBE first (small, above), gcc or clang only if that is not
  enough.
- tcc in Redox' package repository (upstream's `wip/dev/lang/tcc` recipe is
  a stub). Upstream posts are the user's; sessions draft the text.
