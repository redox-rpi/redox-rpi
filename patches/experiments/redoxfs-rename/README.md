# rename on non-root RedoxFS (#57, was experiment `redoxfs-rename`)

**Default since the merge of this branch:** redoxfs `0036` (moved to
`patches/redoxfs/`) is in every image. `--experiment redoxfs-rename` is
still accepted and does nothing, so older commands keep working. This page
stays as the patch's documentation.

Fix for [#57](https://github.com/redox-rpi/redox-rpi/issues/57): `rename`
fails with `ENOENT` on every RedoxFS except the root one, e.g.
`mv /scheme/data/a /scheme/data/b` (found in Pi test 59, where
`ssh-keygen -A` could not move its keys into place).

## Cause

relibc's `renameat2` sends `frename` the target as an absolute path with
its scheme (`/scheme/data/b`). RedoxFS' `frename_internal` checked that the
scheme is its own, but then walked the whole path from the filesystem root,
looking for a directory `scheme`. The root filesystem works because its
paths carry no scheme. `flink` (`validate_new_link`) already removes the
scheme; `0036` does the same for both paths of a rename.

## Results

QEMU raspi3b with a RedoxFS data partition (`qemu-raspi3b.sh --data` from
branch `qemu-data-part`), 2026-10-30:

- Without the fix (`v0.3.0-dev-212-gc9fcc6d+x12cb`): `mv` on
  `/scheme/data` fails with `No such file or directory (os error 2)`, with
  absolute and relative names; on the root filesystem it works.
- With the fix (`v0.3.0-dev-311-g9dcb81b+xd71d`): absolute and relative
  renames on `/scheme/data`, a move into a subdirectory, a move from
  `/scheme/data` to `/home/root` (EXDEV, `mv` copies) and renames on the
  root filesystem all work; the moved file keeps its content.

On the Pi, 2026-10-30, test 83 with `v0.3.0-dev-311-g9dcb81b+xd71d`
([report](../../../docs/test-reports.md#2026-10-30-v030-dev-311-g9dcb81bxd71d)):
**pass**. Absolute and relative renames on `/scheme/data`, a move into a
new directory, `mv` on `/tmp` and `rm -r` all work; 0 of 300 starts failed.
