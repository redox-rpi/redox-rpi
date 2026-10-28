# Experiment: sigentry-x18, relibc built with x18 reserved

For [#1](https://github.com/redox-rpi/redox-rpi/issues/1): rare data
aborts at program start.

## The bug

relibc's aarch64 signal entry (`redox-rt/src/arch/aarch64.rs`) returns to
the interrupted code with

```text
ldp x18, x0, [x18, #16]
br x18
```

The comment there says x18 is the ABI's platform register, so clobbering
it is safe. That only holds when the compiler never uses x18, but the
Redox targets do not reserve it: `rustc --print target-spec-json --target
aarch64-unknown-redox` shows `"features": "+v8a"` and nothing else, and
`ld.so.1` uses x18 in 856 instructions. So when a signal arrives while
x18 holds a live value, the interrupted code continues with x18 set to
its own instruction address.

Experiment [`t1-memmap`](../t1-memmap), QEMU, 700 starts, 8 crashes:

- 7 of the 8 (6 times `pwd`, once `sha256sum`) crash in relibc's dynamic
  linker code (`DSO::relocate`, `Dynamic::symbol_name`), and each of them
  has X18 set to an address in that same code (`0xC04214`..`0xC046FC`).
  In one crash X18 equals ELR.
- The code there uses x18 as a pointer (`ldr x8, [x18, #360]`), reads
  instruction bytes instead of relocation data, and faults on the
  resulting address. That is why FAR looks like instruction bytes.
- At program start, the shell sends SIGCONT to a new foreground job (see
  [#12](https://github.com/redox-rpi/redox-rpi/issues/12)), so a signal
  during the dynamic linker is likely.
- The `ls` crash (ELR `0xAD0C30`, X18 `0`) does not fit this pattern.

## The change

relibc patch `0001` builds relibc (libc.so, ld.so, crt objects, and core,
alloc and compiler_builtins through `build-std`) with `-Zfixed-x18`, and
its C parts with `-ffixed-x18`. Programs and other libraries still use
x18, so this only covers crashes in relibc code. A complete fix needs
either the whole toolchain with x18 reserved or a signal return that
clobbers no register.

```bash
scripts/build-image.sh --experiment t1-memmap --experiment sigentry-x18
```

## Result

`ld.so.1` and `libc.so.6` use x18 in 1 instruction (the `br x18` itself)
instead of 856.

QEMU, 2026-10-28, `ls` and `pwd` 300 times each, `sha256sum` 100 times,
each with output to `/dev/null`:

| Build | Starts | Failed |
|---|---|---|
| `v0.3.0-dev-12-g61a361f+x951b` (`t1-memmap` only) | 700 | 8 (7 with X18 in relibc code) |
| `v0.3.0-dev-13-g8cd57b6+x4697` (`t1-memmap` + `sigentry-x18`) | 700 | 0 |

At the old rate, 0 failures in 700 would happen by chance about 3 times
in 10000. Both runs also show the known `pcid` exception at boot, which
is unrelated.

Not yet tested on a real Pi.
