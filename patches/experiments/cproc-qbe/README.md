# Experiment: cproc-qbe, a second C compiler on the Pi that optimises

For [#327](https://github.com/redox-rpi/redox-rpi/issues/327), topic
[tcc](../../../docs/topics/tcc.md#other-compilers-cproc--qbe). Work in
progress; not in any release image.

Build after `tcc`: `--experiment tcc --experiment cproc-qbe`.

The chain on the Pi, run by the `cproc` driver:

    tcc -E -undef (preprocessor) → cproc-qbe (C11 → QBE IL) → qbe (→ arm64 assembly) → as → ld

- `recipes/wip/dev/other/qbe`: QBE master, replaces upstream's stub.
  Patch `0001`: never x18 as a scratch register (relibc clobbers x18 in
  its signal entry, #127); the register allocator already keeps x18 out.
- `recipes/wip/dev/lang/cproc`: cproc master (new recipe). Patch `0001`:
  a Redox target in `configure` (relibc's `crt0.o`/`crti.o`/`crtn.o`,
  `/lib/ld.so.1`, gcc's integer type macros for relibc's `stdint.h`).
  cproc's own preprocessor is not finished, so the driver runs
  `tcc -E -undef -nostdinc -isystem /usr/include`; `-undef` is tcc patch
  `0005` in experiment `tcc`.
- `recipes/dev/binutils-gdb`: only `as` and `ld` from Redox' binutils
  2.43.1 fork (upstream's recipe builds all of binutils and gdb).
- `cproc.ion` in `/usr/share/tcc-tests/`: the checks and the benchmark
  against tcc and gcc -O2.

## Results

None yet.
