# Experiment: kernel-tlb-debug, what each CPU does when a TLB shootdown stalls

Issue [#207](https://github.com/redox-rpi/redox-rpi/issues/207). The chaos
test's `threads` round (`chaos --kind threads --round-seed
0x4e3014eeaaa65f76`, experiment [`chaos`](../chaos/README.md)) hangs QEMU
raspi3b on the first try. The only line is
`TLB shootdown stalled on CPU #1: acknowledgements 0/1`: one CPU waits
forever for another to acknowledge a TLB shootdown.

Debug only, never for images people use. Kernel `0041` counts the
shootdowns each CPU handles and remembers the ack word (address space) it
last acknowledged on. When a shootdown stalls, the kernel prints that for
every CPU, plus whether the CPU is in the waiting address space's CPU set,
whether its request flag is still set and whether it is in a syscall. It
prints again every 5 s:

- `wants true` on the CPU that owes the ack: it never took the IPI
  (interrupts masked while it spins, or the IPI was lost).
- `wants false` and `last ack` different from the waiting ack word: it
  took the IPI but acknowledged on another address space (it switched in
  between).

It also adds the page, the kind of access and the CPU to the
`Lacks grant` debug line: the Pi printed eight of them right before it
froze in tests 237 and 238.

Kernel `0043` adds the same kind of watch to the seven lock waits in
`src/sync/ordered.rs`: a wait that spins for more than 3 s prints
`lock wait stalled on CPU N: lock 0x… (type)`, then again every 10 s. It
applies on top of default kernel `0052` (the fix from PR #209). Pi test 240 froze once, in a `threads`
round after heavy memory and SD card load, without any line from the
shootdown watch.

```bash
scripts/build-image.sh --experiment chaos --experiment kernel-tlb-debug
```
