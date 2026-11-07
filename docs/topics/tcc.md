# Native C compiler (TinyCC)

The first compiler that runs on Redox on the Pi itself: TinyCC (`tcc`),
about 390 KB, compiles and links in one step, `tcc -run` runs a C file
without writing a binary. Chosen in the Architect session as the first
compiler to try. Issue:
[#247](https://github.com/redox-rpi/redox-rpi/issues/247).

## State

Nothing on `main` yet.

## In progress

Experiment [`tcc`](../../patches/experiments/tcc/README.md) on branch `tcc`:
tinycc `mob` (arm64 backend) with four small patches, cross-compiled for
aarch64 Redox, plus test programs (`tools/tcc-bench`). Pi test
[#278](../test-plan.md) is queued.

**QEMU** (raspi3b, `v0.4.0-dev-689-gb453b397+xd12c`, 2026-11-07):

| Check | Result |
|---|---|
| Hello World, dynamic (`tcc -o hello hello.c`) | pass: compile 230-290 ms, run 100 ms |
| Hello World, static (`tcc -static`) | **fails**: relibc's `libc.a` uses TLS descriptor relocations (`R_AARCH64_TLSDESC_*`, 562-569), which tcc's linker does not know; about 4800 errors, 80-95 s |
| `tcc -run hello.c` | pass: 520-580 ms (instruction cache maintenance from user space works) |
| Benchmark (fib, sieve, mandelbrot) compiled by tcc | pass: compile 0.20 s (4616 lines with headers, 22,851 lines/s), same results as gcc |
| Self-hosting: tcc compiles `tcc.c` (77,143 lines with headers) | pass: 5.5 s (14,092 lines/s), 411 KB of code; the new `tcc2` compiles Hello World, which runs |

QEMU emulates the CPU, so its run times say little about the Pi. In QEMU the
benchmark's tcc code took 504 ms and gcc -O2's 471 ms; fib(32) alone was
89 against 41 ms. Pi numbers come from #278.

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

- Pi test #278: compile and run times on the real Pi.
- Static linking: TLSDESC support in tcc's arm64 linker, or a relibc
  `libc.a` built with traditional TLS (`-mtls-dialect=trad`).
- Ion has no `time`; `timeit` (in `tools/tcc-bench`) fills the gap for now.

## Later

- A second compiler that optimises (gcc or clang for Redox aarch64), for
  programs where tcc's code is too slow.
- tcc in Redox' package repository (upstream's `wip/dev/lang/tcc` recipe is
  a stub). Upstream posts are the user's; sessions draft the text.
