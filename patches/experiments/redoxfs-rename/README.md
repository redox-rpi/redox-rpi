# redoxfs-rename

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
scheme; `0001` does the same for both paths of a rename.

```bash
scripts/build-image.sh --experiment redoxfs-rename
```
