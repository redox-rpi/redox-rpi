# Stability: start crashes, kernel and signal bugs

## State

- relibc patch `0004` fixed the swapped x1/x2 registers in the aarch64
  signal entry, which crashed every third program before v0.1.0; see
  [debugging-random-crashes.md](../debugging-random-crashes.md).
- v0.2.0: 300 of 300 starts on a real Pi 3B+ with `ldso-diagnostics`.
- Kernel patch `0003` (2026-10-28, [#12](https://github.com/redox-rpi/redox-rpi/issues/12), [#5](https://github.com/redox-rpi/redox-rpi/issues/5)): on aarch64, a
  signal that arrived during a syscall made the syscall return its first
  argument. Programs started from the shell died in `relibc_verify_host`
  (`ps` to the terminal, `reboot`). On a real Pi: `ps` 80 of 80,
  `reboot` 5 of 5, 1 failed start in 900 without `ldso-diagnostics`
  (before: 36 of 300). [Details](../../patches/experiments/sigdelivery-x0/README.md).
- Relibc patch `0007` (2026-10-28, [#1](https://github.com/redox-rpi/redox-rpi/issues/1)): relibc's aarch64 signal
  entry returns with `br x18`, but the Redox target does not reserve x18,
  so a signal that arrived while x18 held a value corrupted it. Programs
  died in the dynamic linker at start (data abort). relibc is now built
  with x18 reserved. QEMU 0 of 700 starts fail (before 8 of 700), real Pi
  0 of 1000 at 600 MHz and 0 of 7000 at 1400 MHz (before, #13: 276 of
  5100). [Details](../../patches/experiments/sigentry-x18/README.md).
  Programs and other libraries still use x18, see Next.
- Start crashes on main, 2026-10-30 ([#1](https://github.com/redox-rpi/redox-rpi/issues/1)): today's main
  (`v0.3.0-dev-339-g801d1fe+x9f76`, stable-image experiments) had **0 of
  3000** failed starts on the Pi at 1400 MHz (test 94) and 0 of 1300 in
  QEMU, with no crash dump at all. The stable image `v0.3.0-dev-15`
  (without the x18 fix) had 6 of 1000 the same day (test 92). Its crash
  log, symbolized against its `libc.so` (ld.so loaded at `0xa80000`):
  5 of 6 are one crash, a read fault in `GnuHashTable::find` (ELR
  `0xc15c0c`), called from `DSO::relocate` → `Scope::_get_sym` →
  `DSO::get_sym`. `DSO::relocate` keeps the DSO pointer in x18 and passes
  `x18 + 0x30` (the DSO's hash table) down, so a corrupted x18 gives a
  bad table address. The sixth jumped to address 1 from `load_object`'s
  init function loop (`blr x8`), likely a value relocated with the same
  bad DSO pointer. No other crash type is left on the Pi 3B+.
- Recount on main, 2026-10-31 ([#1](https://github.com/redox-rpi/redox-rpi/issues/1)), after PR #122
  (self-exit-status) and PR #61 (which adds `sigreturn-svc` as an
  experiment, not built here): main cf2025e, `v0.3.0-dev-764-gcf2025e+x9f76` (stable-image experiments),
  QEMU raspi3b: **0 of 5000** starts failed (5 boots of 1000 starts,
  `pi-test.py --crash-log`), crash log empty, no exception in any
  serial log. Before: 0 of 1300 on `v0.3.0-dev-339`. Real Pi, same
  image, test 149: **0 of 3000**, no FATAL or exception on the serial
  console. No start crash is left; #1 can be closed.
- Base `0024` and kernel `0007` (2026-10-29, [#28](https://github.com/redox-rpi/redox-rpi/issues/28), [#32](https://github.com/redox-rpi/redox-rpi/issues/32)): fast input on
  the serial console no longer loses characters (the kernel keeps PL011
  input while it writes to the port), and getty no longer panics when the
  shell reads late (it keeps the input and retries). Real Pi: 20 of 20
  lines of 200 characters at full speed; QEMU: a line of 1050 characters
  4 of 7 runs panicked before, 0 after.
  [Details](../../patches/experiments/serial-rx/README.md). 2026-11-01:
  `0024` reworked so it applies to the newest upstream base too, where
  upstream's own getty fix still stalls the output
  (`docs/upstream/rebase.md`).

## In progress

- [#11](https://github.com/redox-rpi/redox-rpi/issues/11) (branch
  `devshd-hangup`): devshd kept `login` and the shell running when a client
  closed the connection without `exit`, two threads each, so every dropped
  devsh call left a session behind. devshd now hangs up such a session
  (SIGHUP to the terminal's foreground group, then SIGKILL). QEMU: 6
  dropped sessions leave nothing; real Pi (#21): 13 dropped sessions
  leave nothing. Ready to merge.

- [#3](https://github.com/redox-rpi/redox-rpi/issues/3) (v0.4.0, branch `t3-icache`): nothing cleaned the data
  cache and invalidated the instruction cache after user code was loaded.
  Experiment [`icache-sync`](https://github.com/redox-rpi/redox-rpi/tree/t3-icache/patches/experiments/icache-sync) does it in the kernel on `mprotect` to
  executable and at exec. QEMU: 0 of 700 starts fail; on one base built
  with and without it (main merged, 2026-10-30) the loop is about 3 %
  slower, within the host's noise. No known failure needs it: Pi test 28
  had 0 of 1000 at 1400 MHz, the same as main without it (test 94: 0 of
  3000). Pi test 127 measures the cost on the Pi with both builds.

## Next

- x18-safe signal return for programs other than relibc: experiment
  `sigreturn-svc` on main (PR #61), opt-in; no start crash needs it.

## Later

- Relibc patch `0005` (ld.so touches fresh memory) may be dropped: QEMU
  has 0 of 1400 failed starts without it
  ([experiment](https://github.com/redox-rpi/redox-rpi/tree/t2-no-populate/patches/experiments/no-ldso-populate)),
  so [#2](https://github.com/redox-rpi/redox-rpi/issues/2) is closed; needs a Pi test first.
- Prebuilt programs from Redox' aarch64 packages (bash, vim, curl …) stop
  with `brk` at start ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
