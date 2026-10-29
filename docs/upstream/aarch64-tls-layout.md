# aarch64: ld.so places static TLS the x86_64 way

- **What breaks:** on aarch64 (TLS variant I) the linker expects the
  first static TLS block at `tp + align_up(16, p_align)` and
  `R_AARCH64_TLS_TPREL` / static TLSDESC offsets above the thread
  pointer. relibc's `ld.so` lays the blocks out as on x86_64. Results:
  initial-exec TLS of a shared library resolves below `tp`
  (`tp - 0x44`), program TLS aligned to more than 16 reads 0 instead of
  its initial value, and a program and a library can see two copies of
  one variable. Programs with TLS aligned to 16 or less that use no TLS
  of libraries other than libc are not affected (the common case).
- **Reproduce:** `scripts/build-image.sh --experiment relibc-tests`
  (branch [`t14-tls-layout`](https://github.com/redox-rpi/redox-rpi/tree/t14-tls-layout)), then in QEMU
  `tls_probe`, `tls_shared` and `tls_align` (dynamic): they print
  0 instead of 33/22/11 or FAIL.
- **Upstream code:** [`src/ld_so/dso.rs`](https://gitlab.redox-os.org/redox-os/relibc/-/blob/c303135936e6a7e77812f371b3b9d0aa468ec3bc/src/ld_so/dso.rs#L424)
  (TLS offsets, TPOFF and TLSDESC relocations around lines 424, 807,
  1199, 1290), [`src/ld_so/linker.rs`](https://gitlab.redox-os.org/redox-os/relibc/-/blob/c303135936e6a7e77812f371b3b9d0aa468ec3bc/src/ld_so/linker.rs#L849)
  (lines 849 and 910), `src/ld_so/tcb.rs`, `src/ld_so/mod.rs`.
- **Our patch:** experiment
  [`tls-variant1`](https://github.com/redox-rpi/redox-rpi/tree/t14-tls-layout/patches/experiments/tls-variant1)
  (`Master::static_offset` / `static_end`, +16 for TPREL and static
  TLSDESC, other architectures unchanged); tests in experiment
  [`relibc-tests`](https://github.com/redox-rpi/redox-rpi/tree/t14-tls-layout/patches/experiments/relibc-tests).
- **State:** QEMU only, no Pi test yet.
