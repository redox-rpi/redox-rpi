# Kernel: pointers for upstream

Notes for the user, who reports to Redox.

## aarch64: no cache maintenance when user code becomes executable (bug)

Found in the kernel `c951af42` (the commit we pin) and still in upstream
master `1a1295b`: the only `ic ialluis` there is in the start-up of a
secondary CPU (`src/arch/aarch64/device/cpu/boot.rs`).

- **What breaks:** on a real Cortex-A53 (Raspberry Pi 3B+), code that was
  written as data runs stale. Our stress test made a page executable with
  `mprotect` after writing new code into it, and the old code ran in about
  a third of the rounds on a fresh page and about half on a reused one. QEMU
  does not model caches, so it never shows this.
- **Why:** instruction fetch does not see what is still in the data cache.
  The data cache has to be cleaned to the point of unification and the
  instruction cache invalidated before such code runs. Nothing does that
  for user code: not `mprotect` to executable (`ld.so` after relocation),
  not exec (`redox-rt` writes the program through a mapping in the calling
  process, then the `proc:` address space switch starts it), not an `fmap`
  with `PROT_EXEC`, and not a page fault that copies a frame into an
  executable grant. The Cortex-A53's instruction cache is VIPT and may
  alias, so it needs `ic ialluis` from EL1; `ic ivau` from user space would
  not cover the kernel's or another process's mapping.
- **Reproduce:** a Pi 3B+ (or any real aarch64 board) with our
  `tools/icachetest`: `icachetest 10000`; its `new-page` and `same-page`
  tests count rounds that ran old code.
- **Our fix:** experiment `icache-sync` (PR #148), kernel patches `0001`
  (sync on `mprotect` to executable and at the exec address space switch)
  and `0002` (executable `fmap`, and page faults that copy a frame or take
  one from a scheme). Both apply to upstream master `1a1295b` as they are.
- **Tested:** Pi 3B+ test 162 ([report](https://github.com/redox-rpi/redox-rpi/blob/main/docs/test-reports.md#2026-10-31-v030-dev-801-gea8b5c2-without-x8a5e-and-with-icache-sync-x9080-instruction-cache-stress-test)): without the patches
  3146 and 3244 of 10000 `new-page` rounds and 5310 and 5349 `same-page`
  rounds ran stale code; with them 0 in every test on both boots, 0 of 1000
  failed starts, and no measurable cost (the `ls` loop 116.8 s with against
  118.9 s without). QEMU: no exception, the stress test passes.
