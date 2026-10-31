# Experiment: icachetest, the instruction cache stress test (#3)

Adds `icachetest` ([tools/icachetest](../../../tools/icachetest)) to the
image and nothing else. The cache maintenance it checks
([`icache-sync`](../icache-sync/README.md)) is default now (kernel `0010`,
`0011`), so every image should pass it:

```bash
scripts/build-image.sh --experiment icachetest
```

Pi test 162 compared builds of `t3-icache` (PR #148) from before that,
with and without the experiment.

On Redox: `icachetest [rounds]` (default 10000 per test). Each round writes
`movz w0, #n; ret` into a page as data, makes it executable and calls it:

- `new-page`: a fresh page each round, made executable with `mprotect`;
- `same-page`: one page, rewritten each round, `mprotect` in between;
- `file-map`: a new file each round, mapped read-execute; the files stay
  until the end of the test (see below);
- `user-flush`: one read-write-execute page with the maintenance done in
  user space (`dc cvau`, `ic ivau`), as a JIT does.

A round that returns another n ran stale code. It prints one line per
test and `PASS` or `FAIL` (exit status 1). QEMU does not model caches,
so there it always passes; only a real Pi shows the difference.

## RedoxFS maps the data of freed blocks

Found with this test, in QEMU too, so not a cache problem: when a file is
deleted or truncated and a new file gets its disk block, `mmap` of the new
file shows the old file's data. `fsync` before `mmap` does not help.
`ICACHETEST_REUSE=1 icachetest 200` shows it (199 of 200 mappings held old
file data); without it the files stay until the end and every mapping is
right.
