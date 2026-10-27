# Experiment: populate pages before lending them to a scheme

Kernel patch for `Grant::borrow` (`src/context/memory.rs`): owned anonymous
pages of the caller that were never touched are populated in the eager loop,
instead of being left to the page fault handler of the scheme that receives
them.

```bash
scripts/build-image.sh --experiment kernel-borrow-populate
```

**Result:** no effect, 5 of 120 starts crashed as without it (QEMU,
2026-10-26). Rules out the lazy borrow path as the cause of the second
start-crash cause. See [debugging-random-crashes.md](../../../docs/debugging-random-crashes.md),
"Second cause", row 3.
