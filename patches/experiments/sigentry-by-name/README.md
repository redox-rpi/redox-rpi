# Experiment: sigentry-by-name, registers saved by field name

Relibc patch that rewrites how the aarch64 signal entry
(`__relibc_internal_sigentry` in `redox-rt/src/arch/aarch64.rs`) saves and
restores the interrupted code's registers.

Before, it pushed register pairs (`stp x2, x1, [sp, #-16]!`) in an order
that had to match the field order of `ArchIntRegs` by hand. One pair in the
wrong order swapped x1 and x2 in every signal handler return (fixed by
relibc patch `0004`).

Now every register is stored at the offset of the field with its own name:

```text
str x9, [sp, #{ss_regs} + {x9}]     // {x9} = offset_of!(ArchIntRegs, x9)
```

The save and the restore are generated from one register list by a macro
(`sigentry!(x9 x10 … x30)`), and x1-x8, sp, pc, x0 and nzcv go through the
same named offsets. A register can only end up in its own field, whatever
the order of the fields. The stack layout (`SigStack`, then the FP/SIMD
registers) is unchanged; the disassembly was checked field by field.

Also: the spurious-signal path now restores x7 and x8 too. x8 can be
changed there after the realtime signal syscall.

```bash
scripts/build-image.sh --experiment sigregs-test --experiment sigentry-by-name
```

Test: `sigregs` (experiment [`sigregs-test`](../sigregs-test)).

## Results

See [test reports](../../../docs/test-reports.md).
