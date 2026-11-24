# Experiment: cproc-qbe, a second C compiler on the Pi that optimises

For [#327](https://github.com/redox-rpi/redox-rpi/issues/327), topic
[tcc](../../../docs/topics/tcc.md#other-compilers-cproc--qbe). Work in
progress; not in any image (that needs the maintainer's decision).

Build after `tcc`: `--experiment tcc --experiment cproc-qbe`.

The chain on the Pi, run by the `cproc` driver:

    tcc -E -undef (preprocessor) → cproc-qbe (C11 → QBE IL) → qbe (→ arm64 assembly) → as → ld

- `recipes/wip/dev/other/qbe`: QBE master, replaces upstream's stub
  (cloned over `git://`: c9x.me's HTTP clone fails). Patch `0001`: never
  x18 as a scratch register for a store (relibc clobbers x18 in its signal
  entry, #127); the register allocator already keeps x18 out. Patch
  `0002`: float constants read without `fscanf` (relibc bug below).
  Patch `0003`: extern thread-local variables on ELF (initial-exec:
  offset from the GOT plus `tpidr_el0`; GNU ld relaxes it to local-exec in
  a program). Before, qbe stopped with `extern thread unavailable on
  arm64` for every `_Thread_local` with external linkage.
- `recipes/wip/dev/lang/cproc`: cproc master (new recipe). Patch `0001`:
  a Redox target in `configure` (relibc's `crt0.o`/`crti.o`/`crtn.o`,
  `/lib/ld.so.1`, gcc's integer type macros for relibc's `stdint.h`).
  Patch `0002`: a lone `;` in a struct is accepted, as by gcc, clang and
  tcc (relibc's `signal.h` has one in `struct ucontext` on aarch64).
  Patch `0003`: `volatile` loads and stores. QBE has no volatile access, so
  each one gets a call to an empty function local to the module, with the
  object's address: QBE does not merge or drop memory accesses across a
  call, and an object whose address goes to a call stays in memory (a
  volatile local survives `longjmp`). One call per access.
  cproc's own preprocessor is not finished, so the driver runs
  `tcc -E -undef -nostdinc -isystem /usr/include -isystem
  /usr/lib/tcc/include`; `-undef` is tcc patch `0005` in experiment `tcc`.
  tcc's own include dir comes after relibc's, for the headers relibc lacks
  (`stdalign.h`, `tgmath.h`).
- `recipes/dev/binutils-gdb`: `as`, `ld`, `ar`, `nm`, `objdump` and
  `strip` from Redox' binutils 2.43.1 fork (upstream's recipe builds all of
  binutils and gdb, with gmp, mpfr and readline). Built as is, no patch.
  `ranlib` is a 118-byte ion script that runs `ar s` on each archive: the
  real one is `ar` again (978 KB), and a link to `ar` does not work
  (binutils links `ranlib` with its own `is-ranlib.o`). Left out:
  `addr2line`, `c++filt`, `elfedit`, `objcopy`, `readelf`, `size`,
  `strings`.
- `cproc.ion` in `/usr/share/tcc-tests/`: the checks and the benchmark
  against tcc and gcc -O2.
- `/usr/share/cproc-probes/run.ion`: 12 small programs for language
  features, each compiled with cproc and with tcc and run; one `PROBE` line
  per program and compiler (results below).

## Size

On top of tcc (3.8 MB, which provides the preprocessor): `as` 2.1 MB,
`ld` 1.6 MB, `qbe` 212 KB, `cproc` and `cproc-qbe` 140 KB, together
4.2 MB. The archive and object tools (#327 step 2) add 5.5 MB: `objdump`
2.5 MB, `strip` 1.1 MB, `ar` 0.98 MB, `nm` 0.97 MB. Each program is
static and carries its own copy of libbfd.

## relibc findings

- **`fscanf` loses its look-ahead character on a pipe.** A literal that
  does not match (`"_%lf"` on `data`) eats the character, and `%lf` eats
  the one after the number; on a file it works (relibc can seek back).
  qbe read cproc's `data` as `dta`. Worked around in qbe (patch `0002`);
  to fix in relibc.
- `signal.h`: an empty member declaration in `struct ucontext` on aarch64
  (cbindgen output); worked around in cproc (patch `0002`).
- `as` and `ld` print `TODO: getrusage(0, …): not implemented` once per
  run (relibc log line, harmless).

## Results

QEMU, `v0.5.0-dev-25-gd3ca14bc` (2026-11-10): all of `cproc.ion` passes.

| Check | QEMU |
|---|---|
| `cproc -o hello hello.c`, dynamic | 1.6 s, runs |
| `cproc -static -o hello-static hello.c` | 78 s, a 4.2 MB static program that runs: GNU ld links relibc's TLS descriptor relocations |
| benchmark total: cproc / tcc / gcc -O2 | 308 / 475 / 195 ms (QEMU, says little about the Pi) |

QEMU, `v0.5.0-dev-106-gcce8a22a+x88fd` (2026-11-12, with the archive
and object tools): all of `cproc.ion` passes. `cproc -c` builds
`twice.o`, `ar rc` + `ranlib` make `libtwice.a` with a symbol index (`nm -s`:
`twice in twice.o`), `objdump -d` disassembles it, `cproc … -L. -ltwice`
links against it and the program prints `twice(21) = 42`; `strip` takes
it from 4304 to 2528 bytes, it still runs, and `nm` then says `no symbols`.

Real Pi 3B+ with the archive and object tools ([test 359](https://github.com/redox-rpi/pi-tests/blob/main/reports/359.md),
2026-11-12): pass. Every step of the check passes; hello dynamic 422 ms,
static 16.9 s, both run; benchmark totals cproc 1143.5 ms, tcc 1470.0 ms,
gcc -O2 1026.7 ms; pi-test 6/6, no panic. The counted run is on a local
merge with PR #373 (`v0.5.0-dev-108-gf98efe3b+x88fd`), because the #151
stall hit the row's own image. A first run on that image showed `timeit:
Interrupted system call` twice (`waitpid` returned `EINTR`, the child kept
running): [#386](https://github.com/redox-rpi/redox-rpi/issues/386), not
caused by this change.

Real Pi 3B+ ([test 317](https://github.com/redox-rpi/pi-tests/blob/main/reports/317.md),
`v0.5.0-dev-25-gd3ca14bc+x898d`): pass. Hello World compiles in 422 ms
(static 16.8 s, runs); benchmark totals cproc 1052 ms, tcc 1381 ms,
gcc -O2 938 ms. Numbers in the
[topic page](../../../docs/topics/tcc.md#cproc--qbe-on-the-pi).

## Language probes (2026-11-15)

`ion /usr/share/cproc-probes/run.ion`, QEMU. Before: main
(`v0.5.0-dev-258-ga597e2e5`). After: this branch
(`v0.5.1-dev-99-gf583fb23+x4ca7`).

| probe | cproc before | cproc after | tcc |
|---|---|---|---|
| inline asm | fails (not supported) | fails | pass |
| `long double` | fails (not supported) | fails | runs, double precision only |
| VLA | pass | pass | pass |
| `_Thread_local` with pthread | fails (qbe: extern thread) | **pass** (qbe `0003`) | pass |
| `_Complex` | fails | fails | fails |
| statement expression, `__typeof__` | fails | fails | pass |
| computed goto | fails | fails | pass |
| `alloca` | pass | pass | pass |
| C11 (`_Generic`, `alignas`, bit fields, ...) | fails (`stdalign.h`) | **pass** (include dir) | pass |
| struct ABI with relibc, varargs, `qsort` | pass | pass | pass |
| `__attribute__` | pass | pass | pass |
| `setjmp` with a `volatile` local, `sqrt`, `sin` | fails (volatile store) | **pass** (cproc `0003`) | pass |
| **total** | **4 of 12** | **7 of 12** | **10 of 12** |

All of `cproc.ion` still passes on the branch build, with no panic or
exception. What is left is the size of upstream work, not a sprint:
- statement expressions: cproc emits statements while it parses but
  evaluates expressions afterwards, so `({ })` would run at the wrong
  time (even inside a `?:` arm that is not taken);
- computed goto: QBE has no indirect jump;
- inline asm, `long double` (IEEE quad, soft-float calls) and `_Complex`.
