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

0002 is not the cause of the hang below. A QEMU run with 0001 and 0002 still
hung once, and the release list never had more than one block there.

```bash
scripts/build-image.sh --experiment timesync --experiment redoxfs-truncate
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

## Open: a hang that comes with patch 0001 (2026-11-06)

- Pi #248 and #250: with 0001, `pkg install vim` or `pkg remove vim` hung in
  2 to 4 of 9 rounds; without it, none in 6. The root's RedoxFS was waiting
  for requests while `pkg`, `ion` and `devshd` waited for answers. #250
  ruled out the scheduler stall of #157.
- Pi #252, with a counter of the scheme loop (`redoxfs-diag`): RedoxFS had
  answered every call it received (in = out) and was not inside a call. One
  initfs `redoxfs` context stayed runnable with growing CPU time.
- QEMU with 0001 and 0002: 1 hang in 28 rounds. RedoxFS kept answering
  other callers; only its close count stopped.
- Next: Pi #257 with `call-diag`, which shows the scheme each blocked call
  waits on and the user PC of each context.
- On the build host (FUSE, same code): 3000 files written, removed while
  open, then closed. Without 0002, about 24 MB of 36 MB stays allocated
  per round; with 0002 all space comes back.
