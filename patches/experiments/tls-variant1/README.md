# Experiment: tls-variant1, aarch64 static TLS layout

Relibc patch for `ld.so` ([#14](https://github.com/redox-rpi/redox-rpi/issues/14)).
The static TLS blocks of the program and the libraries loaded at start
were placed as on x86_64. On aarch64 (TLS variant I) the linker expects:

- the first block at `align_up(16, p_align)` from the thread pointer,
  every further block at the next aligned offset after the previous one;
- `R_AARCH64_TLS_TPREL` (initial exec) and static TLSDESC as offsets
  **above** the thread pointer.

Before the patch, in QEMU (tests from experiment
[`relibc-tests`](../relibc-tests)):

| Thread-local | Where it was | Initial value read |
|---|---|---|
| program, aligned to 64 or 128 | linker: `tp + align_up(16, align)`, loader copied to `tp + 16` | 0 instead of 33, 5, 7, 9 |
| shared library, initial exec | `tp - 0x44`, below the thread pointer | 0 instead of 22 |
| shared library, TLSDESC | `tp + 0x48` | 0 instead of 11 |
| the programs initial-exec reference to the librarys variable | `tp - 0x48`, the library uses `tp + 0x48` | two copies |

Programs whose TLS is aligned to 16 or less and that use no TLS of
libraries other than libc were not affected; that is the common case.

The patch adds `Master::static_offset` and `Master::static_end` (block
start and end in the static TLS area, 16 bytes above the thread pointer)
and adds 16 for TPREL and static TLSDESC. Other architectures are
unchanged.

Not covered: the thread pointer is only 16-byte aligned (it sits 16 bytes
below a page boundary), so a block aligned to more than 16 is at the right
offset but not at an address aligned to more than 16. aarch64 has no
alignment trap for normal and exclusive accesses up to 16 bytes, so this
only matters for cache-line padding.

```bash
scripts/build-image.sh --experiment relibc-tests --experiment tls-variant1
```

## Results

QEMU, 2026-10-29, `v0.3.0-dev-150-ge231651+x44a9` (with `relibc-tests`):

| Test | Before | With the patch |
|---|---|---|
| `tls_probe` | program TLS 0, library TLS 0, two addresses for one variable | `exe_var` 33 at `tp + 0x40`, `lib_gd` 11 at `tp + 0x50`, `lib_ie` 22 at `tp + 0x54`, program and library agree |
| `tls_shared` | FAIL | PASS, main thread and new thread |
| `tls_align`, dynamic | FAIL | PASS |
| `tls_align`, static | FAIL | FAIL: the static tests link the toolchain's `libc.a`, not this build's (see [`relibc-tests`](../relibc-tests)) |

The rest of the suite is unchanged (no new failures); the image boots and
`errno` (libc's own TLS) still works.
