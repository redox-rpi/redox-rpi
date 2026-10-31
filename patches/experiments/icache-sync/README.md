# Experiment: icache-sync, cache maintenance when user code is loaded

For [#3](https://github.com/redox-rpi/redox-rpi/issues/3).

**Default now:** the two patches are kernel `0010` and `0011` in
[`patches/kernel/`](../../kernel/), in every image (after Pi test 162
showed stale code without them). `--experiment icache-sync` adds nothing
any more; this page keeps the history.

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

User space cannot fix this alone for exec and `mprotect`: the code was
written through another mapping (exec) or the kernel's linear mapping,
and the Cortex-A53's instruction cache is VIPT and may alias, so it needs
`ic ialluis`, which only EL1 may use. Linux does the same for VIPT
instruction caches.

`SCTLR_EL1.UCI` and `UCT` are set (by the bootloader), so a program that
writes code into its own read-write-execute memory (a JIT) can do its own
maintenance with `dc cvau` and `ic ivau`, as on Linux; `icachetest`'s
`user-flush` test does that without a trap.

QEMU does not model caches, and no crash so far was traced to stale
instructions (the start crashes were the x18 bug, #1). This is a
correctness fix for real hardware.

## The change

Kernel patch `0001` (default `0010`):

- `AddrSpace::mprotect`: when pages become executable, clean each mapped
  page to the point of unification through the kernel's linear mapping,
  then `ic ialluis`.
- `proc:` address space switch (the end of exec): the same for every
  executable grant of the new address space, before the context runs in
  it.

Kernel patch `0002` (default `0011`), the other ways code reaches an executable page:

- `mmap` of a file with `PROT_EXEC` (`borrow_fmap`): the scheme filled the
  pages as data;
- a page fault in an executable grant that copies a frame (copy on write)
  or takes a frame from a scheme.

Fresh zeroed frames are left alone: exec fills a new program through such
faults and syncs the whole address space at the switch, and a sync per
fault made the QEMU loop 18 % slower for nothing.

```bash
scripts/build-image.sh --experiment icache-sync
```

## Stress test

[`icachetest`](../icachetest/README.md) writes `movz w0, #n; ret` into a
page as data, makes it executable (`mprotect` on a new page, `mprotect` on
the same page, `mmap` of a file, user-space maintenance on a
read-write-execute page) and counts rounds that return an old n. Build
it into an image with `--experiment icachetest`, with and without this
experiment.

## Result

QEMU, 2026-10-28, `ls` and `pwd` 300 times each, `sha256sum` 100 times,
output to `/dev/null`:

| Build | Starts | Failed | Loop time |
|---|---|---|---|
| `v0.3.0-dev-64-gb173d02+xdef1` (without this patch) | 700 | 0 | 238 s, 244 s |
| `v0.3.0-dev-103-g51c4b57+xe42b` (`icache-sync`) | 700 | 0 | 258 s |

QEMU cannot show whether the maintenance is needed; it only shows that
nothing breaks and roughly what it costs. Not yet tested on a real Pi.

QEMU, 2026-10-30, same loop, the same base built twice (`t3-icache` with
main merged), three runs each, alternating:

| Build | Exceptions | Loop time |
|---|---|---|
| `v0.3.0-dev-469-g7021aa9+x12cb` (without) | 0 | 240 s, 268 s, 193 s |
| `v0.3.0-dev-469-g7021aa9+x12c9` (`icache-sync`) | 0 | 242 s, 254 s, 229 s |

The difference (about 3 %) is within the noise of the shared build host.

Real Pi 3B+, 2026-10-28, test 28: 0 of 1000 starts failed.

Real Pi 3B+, 2026-10-31, test 127 (`0001` only, both builds of one base):
0 of 1000 starts failed with and without it; the `ls` loop is 1.3 %
slower with it (about 2 ms per program start).

QEMU, 2026-10-31, `v0.3.0-dev-801-gea8b5c2` with `icachetest`, without
(`+x8a5e`) and with `icache-sync` (`+x9080`, `0001` and `0002`):

- `icachetest 3000`: all four tests 0 stale rounds in both builds (QEMU
  has no caches to get wrong; this shows the test and the patch work).
- Loop (`ls` and `pwd` 300 times, `sha256sum` 100 times), four rounds
  alternating, no exception: without 266, 288, 258, 302 s; with 306,
  315, 298, 347 s (about 13 %).
- Where the time goes (a diagnostic build counting syncs, boot plus 600
  starts): 1151 `mprotect`, 833 exec, 0 fmap, 16 page faults. The exec
  sync cleans about 900 pages (3.6 MB) per start, because exec copies
  the whole program and `ld.so` on every start; that is `0001`'s cost,
  which test 127 measured on the Pi. In QEMU, `ic ialluis` also throws
  away QEMU's translated code, which a Cortex-A53 does not pay.

Real Pi 3B+, 2026-10-31, test 162 ([report](https://github.com/redox-rpi/redox-rpi/blob/main/docs/test-reports.md#2026-10-31-v030-dev-801-gea8b5c2-without-x8a5e-and-with-icache-sync-x9080-instruction-cache-stress-test)), the same two
builds, A, B, A, B on fresh boots:

| `icachetest 10000` | without, boot 1 | with, boot 1 | without, boot 2 | with, boot 2 |
|---|---|---|---|---|
| `new-page` | **3146** stale | 0 | **3244** | 0 |
| `same-page` | **5310** | 0 | **5349** | 0 |
| `file-map` | 0 | 0 | 0 | 0 |
| `user-flush` | 0 | 0 | 0 | 0 |

**The gap is real on the Cortex-A53, and `icache-sync` closes it.** 0 of
1000 starts failed with both builds, no exception, and no measurable cost:
the `ls` loop took 116.8 s with and 118.9 s without on the first boots.
Upstream note: [docs/upstream/kernel.md](../../../docs/upstream/kernel.md).
