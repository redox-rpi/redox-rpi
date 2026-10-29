# aarch64: the signal entry overwrites x18 when it returns

- **What breaks:** `__relibc_internal_sigentry` returns to the
  interrupted code with `ldp x18, x0, [x18, #16]; br x18`. The Redox
  aarch64 targets do not reserve x18 (Rust `aarch64-unknown-redox` has
  no `+reserve-x18`, gcc no `-ffixed-x18`), and gcc-built packages
  use it (vim, find, bash and others). After any signal, x18 holds a
  relibc address. Also, delivery is enabled again in `inner` before the
  restore runs; x86_64 protects that part with the
  `__relibc_internal_sigentry_crit_*` labels, aarch64 has nothing.
  User space cannot load pc without a free register, so a fix needs the
  kernel or a system-wide reserved x18.
- **Reproduce:** `scripts/build-image.sh --experiment sigregs-test`,
  then `sigregs process -n 2000` in QEMU: "x18 changed after 2
  signals" (`tools/sigregs` on branch [`t1-sigreturn`](https://github.com/redox-rpi/redox-rpi/tree/t1-sigreturn)
  checks x18 on Redox).
- **Upstream code:** [`redox-rt/src/arch/aarch64.rs`](https://gitlab.redox-os.org/redox-os/relibc/-/blob/c303135936e6a7e77812f371b3b9d0aa468ec3bc/redox-rt/src/arch/aarch64.rs#L389)
  lines 389-390 and 404; the kernel's aarch64 SVC handler
  (`src/arch/aarch64/interrupt/`) and `src/context/signal.rs`
  ([kernel](https://gitlab.redox-os.org/redox-os/kernel/-/blob/c951af42c02ba6b7e6355923d46650ea4eadb367/src/context/signal.rs)).
- **Our patch:** experiment
  [`sigreturn-svc`](https://github.com/redox-rpi/redox-rpi/tree/t1-sigreturn/patches/experiments/sigreturn-svc):
  the signal entry restores every register and ends with
  `svc #0x5352`; the kernel sets pc and x0 from sigcontrol. Kernel and
  relibc must change together. Our earlier partial fix builds relibc
  itself with x18 reserved (relibc patch `0007`).
- **State:** QEMU passes (sigregs 3000 signals, prebuilt programs);
  Pi test queued (test plan #36).
