# Experiment: cproc-qbe, a second C compiler on the Pi that optimises

For [#327](https://github.com/redox-rpi/redox-rpi/issues/327), topic
[tcc](../../../docs/topics/tcc.md#other-compilers-cproc--qbe). Work in
progress; not in any release image.

Build after `tcc`: `--experiment tcc --experiment cproc-qbe`.

The chain on the Pi, run by the `cproc` driver:

    tcc -E -undef (preprocessor) → cproc-qbe (C11 → QBE IL) → qbe (→ arm64 assembly) → as → ld

- `recipes/wip/dev/other/qbe`: QBE master, replaces upstream's stub
  (cloned over `git://`: c9x.me's HTTP clone fails). Patch `0001`: never
  x18 as a scratch register for a store (relibc clobbers x18 in its signal
  entry, #127); the register allocator already keeps x18 out. Patch
  `0002`: float constants read without `fscanf` (relibc bug below).
- `recipes/wip/dev/lang/cproc`: cproc master (new recipe). Patch `0001`:
  a Redox target in `configure` (relibc's `crt0.o`/`crti.o`/`crtn.o`,
  `/lib/ld.so.1`, gcc's integer type macros for relibc's `stdint.h`).
  Patch `0002`: a lone `;` in a struct is accepted, as by gcc, clang and
  tcc (relibc's `signal.h` has one in `struct ucontext` on aarch64).
  cproc's own preprocessor is not finished, so the driver runs
  `tcc -E -undef -nostdinc -isystem /usr/include`; `-undef` is tcc patch
  `0005` in experiment `tcc`.
- `recipes/dev/binutils-gdb`: only `as` and `ld` from Redox' binutils
  2.43.1 fork (upstream's recipe builds all of binutils and gdb, with gmp,
  mpfr and readline). Built as is, no patch.
- `cproc.ion` in `/usr/share/tcc-tests/`: the checks and the benchmark
  against tcc and gcc -O2.

## Size

On top of tcc (3.8 MB, which provides the preprocessor): `as` 2.1 MB,
`ld` 1.6 MB, `qbe` 212 KB, `cproc` and `cproc-qbe` 140 KB, together
4.2 MB.

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

Real Pi: row 317 in [pi-tests](https://github.com/redox-rpi/pi-tests/blob/main/queue.md).
