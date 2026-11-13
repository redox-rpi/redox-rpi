# Experiment: redoxfs-truncate, free all blocks of a removed file

RedoxFS did not free the last, partly filled record of a file when the file
was truncated or removed ([#197](https://github.com/redox-rpi/redox-rpi/issues/197)).
`truncate_node_inner` freed the records up to `old_size / record`, rounded
down. A record is up to 128 KiB, so a removed file smaller than that kept
all its blocks, and a larger one kept its tail. The space stayed lost after
a remount. Files whose size is an exact multiple of the record size were
not affected.

On the network-boot image (222 MiB root, about 136 MB free) each
`pkg install vim` plus `pkg remove vim` lost about 13 MB, and the eighth
round failed with `No space left on device`. vim has 2223 files, most of
them small.

The patch rounds the upper end up (`old_size.div_ceil(record)`). redoxfs is
pinned in `config/upstream.lock`. Upstream `master` (5ba45a6, 2026-10-31)
has the same code.

Patch 0002 fixes a second, older bug. When a file is removed while it is
still open, RedoxFS puts it on a release list and frees it when the last
handle closes. `release_unused_nodes` relinked that list in the wrong
direction: with more than one list block (more than about 1000 such files)
the files in the other blocks were never freed, and one block kept pointing
at a deallocated block. 0002 walks the list oldest block first and writes a
block again when its link changed.

The hang that came with 0001 (below) is a kernel futex race, not RedoxFS:
build with experiment `futex-wait-lock` too.

```bash
scripts/build-image.sh --experiment timesync --experiment redoxfs-truncate --experiment futex-wait-lock
```

Space that was lost before the fix stays lost on that filesystem. On
network and direct boot the root is new at every boot, so nothing is left
over there. On an SD card with an older data partition the lost blocks stay
allocated.

## How it was found and checked (2026-11-04)

- QEMU, image `v0.4.0-dev-368-g58515233+x12cb` (main, `timesync`), vim from
  a local copy of `static.redox-os.org`: free space 136.8 MB at the start,
  then after each `pkg remove vim` 111.8, 98.4, 85.3, 71.9, 58.7 MB.
  `/tmp/pkg_download` keeps the last download of each package (11.8 MB for
  vim and its dependencies) but does not grow, because the next download
  of the same package replaces the file.
- On the build host, the same root partition mounted with the host's
  `redoxfs` (FUSE, same code): `df` counted 13 MB more than `du` after one
  round, so the space was not in any file. Writing and removing 500 files
  of one size lost: nothing at 100 bytes (stored in the node), the whole
  file at 4 KiB to 100 KB, about 65 to 130 KB per file above that.
- With the patch: the same test loses nothing, and two rounds of writing
  and removing vim's 2223 files end within 0.2 MB of the start.

## The hang that came with patch 0001: a kernel futex race (2026-11-06)

- Pi #248 and #250: with 0001, `pkg install vim` or `pkg remove vim` hung in
  2 to 4 of 9 rounds; without it, none in 6. The root's RedoxFS was waiting
  for requests while `pkg`, `ion` and `devshd` waited for answers. #250
  ruled out the scheduler stall of #157.
- Pi #252, with a counter of the scheme loop (`redoxfs-diag`): RedoxFS had
  answered every call it received (in = out) and was not inside a call. One
  initfs `redoxfs` context stayed runnable with growing CPU time.
- QEMU with 0001 and 0002: 1 hang in 28 rounds. RedoxFS kept answering
  other callers; only its close count stopped.
- Pi #257 with `call-diag`: pkg waited on ptyd or netstack while those sat
  idle; a QEMU hang on the same image had both pkg threads blocked in
  `futex`. The pinned kernel (c951af42) checks the futex word in
  `FUTEX_WAIT` and only then queues the waiter, so a `FUTEX_WAKE` in between
  wakes nobody. 0001 makes pkg faster (fewer blocks to write), which made
  the race more likely.
- Pi #258 with the upstream fix backported (experiment `futex-wait-lock`,
  upstream 57ccc8f and 5805059): **10 of 10 rounds with no hang** (3 hangs in
  6 without it), free space after each remove flat at 124.5 to 124.7 MB.
- On the build host (FUSE, same code): 3000 files written, removed while
  open, then closed. Without 0002, about 24 MB of 36 MB stays allocated
  per round; with 0002 all space comes back.

## Results on today's main (2026-11-12, QEMU)

`pkg install --yes vim` then `pkg remove --yes vim` from a local mirror,
`df /` after each step, QEMU `--usb-net`, about 12.5 min per round.

| after remove | before (`v0.5.0-dev-184-g07304cce+x097a`, main + `timesync`) | after (`v0.5.0-dev-185-g841ffe14+xb56b`, + `redoxfs-truncate` + `futex-wait-lock`) |
|---|---|---|
| round 1 | 88384 KB free | 101352 KB |
| round 2 | 75164 KB | 101424 KB |
| round 3 | 61940 KB | 101496 KB |
| round 4 | 48712 KB | 101560 KB |
| round 5 | 35472 KB | 101612 KB |
| round 6 | install fails: `No space left on device (os error 28)` | 101408 KB |
| round 7 | | 101480 KB |

Before: about 13.2 MB lost per round. After: flat within 0.3 MB over 7
rounds, every install and remove `rc=0`, no hang, no panic.

Pi 3B+, row [390](https://github.com/redox-rpi/pi-tests/blob/main/reports/390.md),
2026-11-13, on the PR's image `v0.5.0-dev-185-g841ffe14+xb56b`: **pass**.
10 of 10 rounds `rc=0`, no hang; free after each remove 101364–101608 KiB
(spread 244 KiB), install 300–313 s, remove 20–21 s; pi-test all pass, no
panic. The first round ends 12.2 MB below the start (113520 KiB) and then
stays flat: that is `/tmp/pkg_download`, which keeps the last download of
vim and its dependencies (11.8 MB, see above).
