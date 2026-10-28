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

## In progress

- [#28](https://github.com/redox-rpi/redox-rpi/issues/28) the serial console loses characters of fast input: the
  kernel cannot take the PL011 receive interrupt while it writes to the
  port, and the 16-byte receive FIFO overflows. Experiment
  [`serial-rx`](https://github.com/redox-rpi/redox-rpi/tree/serial-rx/patches/experiments/serial-rx)
  on branch `serial-rx`: the kernel keeps the input during writes, and
  getty no longer panics when the PTY does not take input at once. Passes
  in QEMU; Pi test #19.

## Next

- [#1](https://github.com/redox-rpi/redox-rpi/issues/1) complete fix: programs and libraries other than relibc still use
  x18, so the signal entry can still corrupt it there. Either reserve x18
  in the whole toolchain (Rust target, GCC `-ffixed-x18`, all packages
  rebuilt) or return from the signal entry without clobbering a register.
- [#2](https://github.com/redox-rpi/redox-rpi/issues/2) its kernel cause; [#3](https://github.com/redox-rpi/redox-rpi/issues/3) instruction cache maintenance.
- [#11](https://github.com/redox-rpi/redox-rpi/issues/11) devshd leaves `login` processes behind.

## Later

- Prebuilt programs from Redox' aarch64 packages (bash, vim, curl …) stop
  with `brk` at start ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
