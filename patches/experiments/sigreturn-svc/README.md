# Experiment: sigreturn-svc, a signal return that clobbers no register

For [#1](https://github.com/redox-rpi/redox-rpi/issues/1), the complete
fix after relibc patch `0007`.

**Default patches now** (relibc `0013`, kernel `0009`;
[#127](https://github.com/redox-rpi/redox-rpi/issues/127), Pi test 45).
The name is still accepted and does nothing.

## The bug

relibc's aarch64 signal entry returned to the interrupted code with
`ldp x18, x0, [x18, #16]; br x18`. x18 is an ordinary register for the
Redox targets, so any code could continue with x18 set to its own
address. Relibc patch `0007` builds relibc with x18 reserved, which fixed
the start crashes in the dynamic linker, but programs and other libraries
still use x18 (about 900 instructions in `ld.so` alone before `0007`).

A second gap: `inner` enabled signal delivery again before the assembly
at the end of the signal entry restored the registers. x86_64 protects
that part with `__relibc_internal_sigentry_crit_*` labels; aarch64 had
nothing, so a second signal there could run while x18 was the base
register of the restore.

## The change

- **relibc** (`redox-rt`): `inner` writes the saved pc and x0 into the
  sigcontrol struct and, instead of enabling delivery, sets control flag
  `4` (SIGRETURN_UNINHIBIT). The end of the signal entry restores every
  register, including x18, nzcv, sp and the FP/SIMD state, and ends with
  `svc #0x5352`. The spurious-signal path does the same.
- **kernel**: an `svc` with immediate `0x5352` is not a syscall. The
  kernel sets pc and x0 from the sigcontrol struct, clears flag `4` and
  INHIBIT_DELIVERY, and runs `signal_handler`, so a signal that became
  deliverable during the handler is delivered right away (what
  `arch_ret_to_sig` did before).
- **sigregs**: `--x18` checks x18 too (it failed on Redox before these
  patches).

Kernel and relibc must come together: the new relibc on an old kernel
would run `svc #0x5352` as a syscall.

Not covered: when the trampoline was entered with signals explicitly
disabled and a signal is pending at the end, the signal is no longer
delivered at once; it stays pending until delivery is enabled again.

Before it became default patches:

```bash
scripts/build-image.sh --experiment sigreturn-svc
scripts/build-image.sh --experiment sigreturn-svc --experiment sigregs-test
```

## Result

QEMU, 2026-10-29, with `sigregs-test` and `sigentry-thread-signals`:

| Build | `sigregs process -n 3000` (x18 checked) | `sigregs thread -n 3000` (x18 checked) | 700 starts (`ls`, `pwd`, `sha256sum`) |
|---|---|---|---|
| `v0.3.0-dev-137-g6988886+xae3b` (without) | FAIL, x18 changed after 3 signals | FAIL, x18 changed after 2 signals | 0 failed |
| `v0.3.0-dev-138-ge543996+xbb89` (with) | PASS | PASS | 0 failed |

A first run with the experiment showed the same FAIL because `sigregs` was
not relinked against the new `libc.a` (fixed in `build-image.sh`, e543996).
The sigregs binary of the passing build has `svc #0x5352` twice and no
`br x18`. Both builds show the known `pcid` exception under QEMU.
