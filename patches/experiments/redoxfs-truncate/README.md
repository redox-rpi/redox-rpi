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
