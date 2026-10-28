# Stability: start crashes, kernel and signal bugs

## State

- relibc patch `0004` fixed the swapped x1/x2 registers in the aarch64
  signal entry, which crashed every third program before v0.1.0; see
  [debugging-random-crashes.md](../debugging-random-crashes.md).
- v0.2.0: 300 of 300 starts on a real Pi 3B+ with `ldso-diagnostics`.

## In progress

- **Signal delivery during a syscall** ([#12](https://github.com/redox-rpi/redox-rpi/issues/12), [#5](https://github.com/redox-rpi/redox-rpi/issues/5)): on
  aarch64, a signal that arrived during a syscall made the syscall return
  its first argument. Kernel experiment `sigdelivery-x0` on branch
  `t12-auxv-dump` fixes it on a real Pi: `ps` 80 of 80, `reboot` 5 of 5,
  1 failed start in 900 without `ldso-diagnostics` (before: 36 of 300).
  Next: to `main` as kernel patch `0003`.

## Next

- [#1](https://github.com/redox-rpi/redox-rpi/issues/1) the remaining rare start crash (a data abort, 1 in 900 on the Pi).
- [#2](https://github.com/redox-rpi/redox-rpi/issues/2) its kernel cause; [#3](https://github.com/redox-rpi/redox-rpi/issues/3) instruction cache maintenance.
- [#11](https://github.com/redox-rpi/redox-rpi/issues/11) devshd leaves `login` processes behind.

## Later

- Prebuilt programs from Redox' aarch64 packages (bash, vim, curl …) stop
  with `brk` at start ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
