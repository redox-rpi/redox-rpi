# Experiment: icache-sync, cache maintenance when user code is loaded

For [#3](https://github.com/redox-rpi/redox-rpi/issues/3).

## The gap

On AArch64, instruction fetch does not see data that is still in the data
cache. Code written as data must be cleaned to the point of unification
(`dc cvau`) and the instruction cache invalidated before it runs. Nothing
did that for user code:

- **ld.so** maps a library read-write, copies and relocates it, then makes
  the code pages executable with `mprotect`.
- **exec** (`redox-rt/src/proc.rs`) maps the new program's segments
  executable in the new address space, writes the code through a second
  mapping in the calling process, and then switches the context to the
  new address space.

User space cannot fix this alone: `SCTLR_EL1.UCI` is not set, and the
Cortex-A53's instruction cache is VIPT and may alias, so even
`ic ivau` by address would not be enough; it needs `ic ialluis`, which only
EL1 may use. Linux does the same for VIPT instruction caches.

QEMU does not model caches, and no crash so far was traced to stale
instructions (the start crashes were the x18 bug, #1). This is a
correctness fix for real hardware.

## The change

Kernel patch `0001`:

- `AddrSpace::mprotect`: when pages become executable, clean each mapped
  page to the point of unification through the kernel's linear mapping,
  then `ic ialluis`.
- `proc:` address space switch (the end of exec): the same for every
  executable grant of the new address space, before the context runs in
  it.

```bash
scripts/build-image.sh --experiment icache-sync
```

## Result

Not run yet.
