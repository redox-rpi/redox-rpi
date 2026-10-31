# Experiment: icachetest, the instruction cache stress test (#3)

Adds `icachetest` ([tools/icachetest](../../../tools/icachetest)) to the
image and nothing else, so an image with and one without
[`icache-sync`](../icache-sync/README.md) can run the same test.

```bash
scripts/build-image.sh --experiment icachetest                         # without
scripts/build-image.sh --experiment icachetest --experiment icache-sync # with
```

On Redox: `icachetest [rounds]` (default 10000 per test). Each round writes
`movz w0, #n; ret` into a page as data, makes it executable and calls it:

- `new-page`: a fresh page each round, made executable with `mprotect`;
- `same-page`: one page, rewritten each round, `mprotect` in between;
- `file-map`: a file rewritten each round and mapped read-execute;
- `user-flush`: one read-write-execute page with the maintenance done in
  user space (`dc cvau`, `ic ivau`), as a JIT does.

A round that returns another n ran stale code. It prints one line per
test and `PASS` or `FAIL` (exit status 1). QEMU does not model caches,
so there it always passes; only a real Pi shows the difference.
