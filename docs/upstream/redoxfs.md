# RedoxFS: pointers for upstream

Notes for the user, who reports to Redox. Both problems are in RedoxFS
`c988cb8` (the commit we pin) and still in upstream master `db847d8`
(`src/mount/redox/scheme.rs` and `src/transaction.rs` unchanged).

## `rename` on a RedoxFS other than the root one

- **What breaks:** `mv /scheme/data/a /scheme/data/b` fails with
  `No such file or directory (os error 2)`, with absolute and relative names,
  on every RedoxFS mounted under a scheme other than the root filesystem.
  Found when `ssh-keygen -A` could not move its keys into place (our #57).
- **Why:** relibc's `renameat2` sends `frename` the target as an absolute
  path with the scheme (`/scheme/data/b`). `frename_internal`
  (`src/mount/redox/scheme.rs:738`) checks that the scheme is its own
  (lines 766–776), then walks the whole path from the filesystem root
  (`TreePtr::root()`, lines 784–788), so it looks for a directory `scheme`.
  `validate_new_link` (line 609, used by `flink`) already strips the own
  scheme (lines 641–645). The root filesystem works only because its paths
  carry no scheme.
- **Reproduce:** QEMU raspi3b with a RedoxFS data partition mounted as
  `/scheme/data`: `echo x > /scheme/data/a; mv /scheme/data/a /scheme/data/b`.
- **Our fix:** `patches/redoxfs/0036` (PR #72): strip the own scheme from
  both paths in `frename_internal`, like `validate_new_link`.
- **Tested:** QEMU and Pi 3B+ test 83.

## Small writes write about 28 times the data

- **What breaks:** a file written in 4 KiB `write` calls sends about 28
  times its size to the disk (QEMU: 1 MiB file, 28.4 MiB written). The same
  on x86_64. On a Pi SD card, copying 1 MiB with `dd bs=4k` takes 12.7 s.
- **Why:** every `write` call is one transaction: `write`
  (`src/mount/redox/scheme.rs:933`) calls `self.fs.tx(...)` (line 943), and
  `FileSystem::tx` (`src/filesystem.rs:242`) commits at its end. A record
  is up to 128 KiB (`RECORD_LEVEL` 5, `src/lib.rs:16`) and copy-on-write, so
  each 4 KiB write rewrites the record as large as its data so far (21.3
  times for a whole record), plus the node, the tree path, the allocator log
  block and the header (about 7 blocks, another 7 times).
- **Reproduce:** `scripts/redoxfs-write-amp.py` (QEMU, counts the bytes
  written to the disk image for 4 KiB, 64 KiB and 1 MiB calls).
- **Related upstream work:** RedoxFS
  [#51](https://gitlab.redox-os.org/redox-os/redoxfs/-/work_items/51),
  [#57](https://gitlab.redox-os.org/redox-os/redoxfs/-/work_items/57).
- **Our workaround:** `patches/redoxfs/0037`–`0039` (PR #73): the scheme
  collects consecutive small writes of one handle and writes whole records,
  at the latest after 2 s. 4 KiB writes: 28.4 → 1.3 times. On-disk format
  unchanged. A fix in RedoxFS itself might rather keep one transaction open
  across calls.
- **Tested:** QEMU, Pi 3B+ tests 46, 70 and 121 (no effect on large writes).
