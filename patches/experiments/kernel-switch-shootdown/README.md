# Experiment: kernel-switch-shootdown, context switch handles TLB shootdowns while it waits

Issue [#207](https://github.com/redox-rpi/redox-rpi/issues/207): the chaos
test's `threads` round froze QEMU and the Pi. With experiment
[`kernel-tlb-debug`](../kernel-tlb-debug/README.md) and the QEMU monitor
(`info registers -a`), all four CPUs had interrupts masked:

- one CPU in `Flusher::flush`, holding an address-space lock, waiting for a
  TLB shootdown acknowledgement;
- the CPU that owed it in `direct_switch_to` → `RwLock::write_arc` →
  `spin::RwLock::try_write_internal`, spinning on a context lock. Its
  shootdown request flag stayed set, and its handled count did not move.

`write_arc` was the only lock wait in `src/sync/ordered.rs` that did not
call `maybe_handle_tlb_shootdown()` while spinning, so the two CPUs waited
for each other forever. Kernel `0042` makes it wait with `try_write` and
handle shootdowns in the loop, like the other lock waits there.

```bash
scripts/build-image.sh --experiment chaos --experiment kernel-switch-shootdown
```
