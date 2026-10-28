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

## In progress

- [#1](https://github.com/redox-rpi/redox-rpi/issues/1) (branch
  `t1-data-abort`): likely cause found. relibc's aarch64 signal entry
  returns with `br x18`, but the Redox target does not reserve x18, so a
  signal that arrives while x18 holds a value corrupts it. 7 of 8 start
  crashes in QEMU have X18 set to an address in relibc's dynamic linker.
  Experiment [`sigentry-x18`](../../patches/experiments/sigentry-x18/README.md)
  builds relibc with x18 reserved: 0 of 700 starts fail in QEMU (before:
  8 of 700). Not yet tested on a real Pi. Programs and other libraries
  still use x18.

## Next

- [#2](https://github.com/redox-rpi/redox-rpi/issues/2) its kernel cause; [#3](https://github.com/redox-rpi/redox-rpi/issues/3) instruction cache maintenance.
- [#11](https://github.com/redox-rpi/redox-rpi/issues/11) devshd leaves `login` processes behind.

## Later

- Prebuilt programs from Redox' aarch64 packages (bash, vim, curl …) stop
  with `brk` at start ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
