# Experiment: t12-auxv-dump, stack layout when the filetable fd is missing

Diagnostics for [T12](../../../docs/tasks.md#t12-programs-fail-at-start-with-no-filetable-fd-present-qemu-pi):
dynamically linked programs stop in `relibc_start_v1` at
`expect("no filetable fd present")`, although `ld.so` found the same auxv
entry just before.

## Idea

`fexec_impl` in `redox-rt/src/proc.rs` pushes `AT_REDOX_FILETABLE_FD`
last, so it is the **first** auxv pair, right after the NULL that ends
envp. `AT_REDOX_THR_FD` is the fifth pair. If `Stack::auxv` starts one
pair too late, only the filetable fd is missing, which is what T12 shows.

The patch replaces the `expect` with a stop at `brk #0x7a`
(`ESR_EL1` `0xF200007A`), with the stack layout in the registers the
kernel prints:

| Register | Content |
|---|---|
| `x0` | `sp` handed to `relibc_start_v1` |
| `x1` | argc |
| `x2` | number of env entries up to the first NULL |
| `x3` | auxv start as `Stack::auxv` computes it |
| `x4`..`x17`, `x20`..`x28` | the 23 words from `x3 - 24` on |

Normally `x6` is 0 (end of envp), `x7` is 45 and `x8` the filetable fd,
then `x9` 47 (`AT_REDOX_CWD_FD`), `x11` 44 (`AT_REDOX_NS_FD`),
`x13` 43 (`AT_REDOX_PROC_FD`), `x15` 42 (`AT_REDOX_THR_FD`).

## Build

```bash
scripts/build-image.sh --experiment ldso-diagnostics --experiment timesync --experiment t12-auxv-dump
```

The same set as v0.2.0, plus this one. Then start `ps`, `reboot` or
`timesync -n` until one stops, and read the dump in the serial log or in
QEMU's log.

## Result

QEMU (`v0.2.0-4-g8405d0c+xfc05`): no `brk #0x7a` in 76 failed starts. The
crash was never at this `expect`: the registers show `x30` still in the
program's start code, `x8` = 158 and `x0`..`x4` = `-1`. The jump to the
`brk #1` came from `relibc_verify_host` right at the top of
`relibc_start_v1`, whose abort shares that `brk #1`. Cause and fix: see
[`sigdelivery-x0`](../sigdelivery-x0/README.md). This experiment is not
needed any more.
