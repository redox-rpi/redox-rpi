# Debugging: programs fail at random (open)

**State 2026-10-26, evening. Not solved.** This page is the handover for
whoever continues: what is known, what was ruled out and how, what is left
in the tree from the experiments, and what to try next.

## The problem

On Redox for aarch64, roughly every third or fourth program started from the
shell ends at once: no output, exit code 1. The same command run again
usually works. Hit are dynamically linked programs: `ls`, `cat`, `cmp`,
`sha256sum` (uutils), `cmp` (diffutils), `kibi`, our own `reboot`. Bigger
prebuilt programs (bash, nano, vim, curl, …) fail more often, or always,
with a `brk` exception.

- Happens in QEMU `raspi3b` exactly as on the real Pi 3B+: not a hardware or
  driver problem.
- Was already there in Redox' own daily aarch64 image of 2026-10-24, also on
  QEMU `virt` with the unmodified kernel (see "Bug 2" in
  [status.md](status.md)). An upstream bug, not caused by our patches.
- Upstream has nothing newer that could fix it: `base` main is still
  `eddcaa2`; relibc master has only unrelated commits after `841d29c`.

## How to reproduce

On the build host (`ssh zehrer@homenodellm`, repository `~/redox-rpi`), after
`scripts/build-image.sh`:

```bash
scripts/qemu-run.sh 'for i in 1..=15; sha256sum /etc/issue > /tmp/o$i; echo $i rc=$?; end'
grep -E 'ELR_EL1|ESR_EL1|NAME' images/qrun.log
```

[`scripts/qemu-run.sh`](../scripts/qemu-run.sh) boots the network boot
image in QEMU, runs the commands through devshd and stops QEMU; the serial
console with the kernel's crash dumps lands in `images/qrun.log`. Typical
result: 3 to 5 of 15 runs with `rc=1`, and one `UNHANDLED EXCEPTION` per
failed run in the log.

Notes on the tools:

- Ion syntax: `for i in 1..=15; …; end`, stderr redirect is `^>`, not `2>`.
- devsh sometimes swallows a command's output. Redirect to a file and check
  the exit code instead of trusting what devsh shows.
- The interesting output is in the kernel log, not in the shell.
- `pkill -f pattern` over ssh kills the ssh session itself when the pattern
  also matches its own command line. Use `pkill -f "[b]uild-image"`.

## Where it fails

Every failed run ends in `relibc_start_v1` (relibc `src/start.rs`), the C
runtime start of the program, called from the program's `_start` right after
the dynamic loader:

```
ESR_EL1: F2000001          brk #1: a Rust panic without a way to print it
X30:     0xEF048           inside the program's _start
```

Disassembling the prebuilt and later our own `libc.so` (`ld.so` and `libc.so`
are one file, mapped at `0xa80000`) shows the `brk` right after
`core::option::expect_failed` with a 23-byte message:
**`no filetable fd present`**:

```rust
if redox_rt::current_filetable().fd().is_none() {
    let filetable_fd = get_auxv_raw(sp.auxv(), AT_REDOX_FILETABLE_FD)
        .expect("no filetable fd present");
```

So two things are true in a failed run:

1. `FILETABLE` (a static in redox-rt, inside `libc.so`) is still empty,
   although `ld.so` (same object, same static) sets it up in
   `src/ld_so/start.rs` from the same auxiliary vector entry before it jumps
   to the program.
2. The auxiliary vector the program sees has no `AT_REDOX_FILETABLE_FD` (45).
   That entry is pushed last by the exec code in the parent
   (`redox-rt/src/proc.rs`, `push(AT_REDOX_FILETABLE_FD)`), so it is the
   **first** auxv entry, directly after the NULL that ends `envp`.

Other crash sites seen before the loader change below, in the same runs:
`memcpy` storing to an unmapped address (`ESR 0x92000047`), `fstat`
storing through a bad pointer, and once `ld.so` panicking with
`non UTF-8 ELF symbol name` (`src/ld_so/dso.rs:171`) and
`unresolved symbol … for soname libonig.so.5`. That looks like general
corruption of data a new process starts with, not one wrong value.

## Ruled out

| Hypothesis | Experiment | Result |
|---|---|---|
| Our SMP patch (4 cores, spin-table, BCM2836 IPIs) | Kernel that starts only the boot CPU | Same failure rate with `CPUs: 1` |
| Stale TLB entries when the parent maps the child's pages | `invalidate()` does `tlbi vmalle1is`, and every `Flusher::flush` flushes the whole local TLB | Same failure rate |
| Timing: the previous child's exit or `SIGCHLD` disturbs the next start | `sleep 2` between runs | Same failure rate |
| The previous child's address space or ASIDs | Read the code: aarch64 uses no ASIDs, `set_table` flushes everything | Not a candidate |
| ld.so maps the file shared and relocates in place | Read the code: `ld.so` maps the file read-only and copies into anonymous memory; exec copies segments into anonymous memory too | Not a candidate |
| Corrupted file mappings (`fmap` from redoxfs) | relibc patch 0002: `ld.so` reads shared objects with `pread` instead of `mmap` | Fewer crash kinds (no more `memcpy`/`fstat`/symbol name crashes), but the same rate of `no filetable fd present` |
| Kernel/userland version skew (prebuilt relibc newer than our kernel) | Compared the kernel ABI change "Sched affinities" (`b313fa4`): `CpuSet` keeps the size of the old mask | Not a candidate |
| Floating point registers lost in `fork()` | relibc patch 0001, see below | A real bug, fixed, but the failures stay |
| FP/SIMD state on context switch, exception entry | Read the code: `fp_save`/`fp_load` save q0-q31, fpcr, fpsr; the kernel is built without NEON; exception entry saves x0-x30 | Looks right |

## Changes in the tree from this hunt

relibc is now built from source (`relibc = "source"` in
`config/raspi3bp-sdcard.toml`, `relibc` in the patch loop of
`scripts/build-image.sh`), with two patches in
[`patches/relibc/`](../patches/relibc/) (branch `redox-rpi` in
`upstream/relibc`, base `841d29c`):

| Patch | Keep? |
|---|---|
| `0001` aarch64: keep d8-d15 across fork. `__relibc_internal_fork_wrapper` saved only x19-x30 (the source says `//TODO: store floating point regs`), so the child continued with the d8-d15 of whatever ran on its CPU before | **Keep.** A real bug, even if not this one |
| `0002` ld.so: read shared objects instead of mapping them | Kept for now (in v0.1.0-alpha): it removes the `memcpy`, `fstat` and symbol name crashes. Revisit once the cause is known; drop if file mappings turn out fine |

A third, diagnostic patch (next section) was removed again before the
v0.1.0-alpha build; it is in the history of this page's commit
(`9199d96`, `24e8eef` in this repository).

The build host's kernel source (`recipes/core/kernel/source`) has no
experiments left in it; only our patches.

## The diagnostic that did not work, and why that is interesting

Patch 0003 replaces the `expect` with a function that loads the stack
contents (stack pointer, argc, envp count, the first auxv words) into
registers and then faults, so that the kernel's crash dump shows them.

- First version: `brk #2`. No dump with `ESR F2000002` appeared; instead the
  process crashed with `brk #1` on the instruction **after** the call to the
  diagnostic function, which is declared `-> !` and ends in a `noreturn`
  `asm!`.
- Second version: a load from address 8 before the `brk #2`. Again no dump
  with `FAR 0x8`, again `brk #1` after the call.

So a fault in this early phase does not end up as an unhandled exception;
something resumes the process and it "returns" from a function that cannot
return. Probably the signal path: the kernel delivers `SIGSEGV`/`SIGTRAP`
through redox-rt's signal trampoline, which at this point in the start runs
with state inherited from the parent. This may only be a side effect, but
signal delivery during program start is a good candidate for the original
corruption too.

## Ideas for the next steps

1. **Check in the parent.** In `redox-rt/src/proc.rs`, after the auxv and
   the strings are pushed, map the child's stack page again and compare
   with what was written; print a mismatch to stderr. The process doing the
   exec is the forked shell, which can still print. This tells whether the
   data is wrong when written or gets lost afterwards.
2. **Print from the child without a file table.** E.g. a raw `SYS_WRITE` to
   the kernel debug scheme, or encode the values in the exit status, instead
   of faulting.
3. **Signals during start.** Check whether a signal is pending or delivered
   while ld.so and `relibc_start_v1` run (e.g. `SIGCHLD` inherited by the
   forked shell, or the sigprocmask/sigignmask from the auxv), and what the
   aarch64 `__relibc_internal_sigentry` does to the stack and to x0 in that
   phase.
4. **x86_64 comparison.** Run the same loop in an x86_64 Redox image from the
   same day. If it is clean, diff the aarch64 and x86_64 paths in redox-rt
   (`arch/aarch64.rs` vs `arch/x86_64.rs`): fork wrapper, sigentry, exec
   stack setup.
5. **Does `ld.so` reset `FILETABLE`?** It "loads" `libc.so.6 (aka ld.so)` a
   second time in its object list; check that this never re-initializes
   `.data`/`.bss` of the running object, and that `ld.so` does not consume
   or overwrite the auxv entries (it moves argv/envp/auxv in place only when
   run as `ld.so prog`, `adjust_stack`).

A fresh pair of eyes (another person or model) should start with this page,
the reproducer above and step 1.
