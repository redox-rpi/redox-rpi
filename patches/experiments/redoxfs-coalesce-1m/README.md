# Experiment: collect up to 1 MiB of small writes (redoxfs)

On top of redoxfs `0037`–`0039` (PR #73, write collection). Those collect
consecutive small writes of a handle and write them one record (128 KiB) at
a time. Each record is then its own transaction, with its own node, tree,
allocator log and header writes. On the Pi (test 175), 4 MiB written in
4 KiB calls became 275 disk requests of 37 blocks on average, and the
small ones took about as much card time as the records: 4 KiB writes ran
at 2.7–3.9 MB/s, 1 MiB writes at 5–11 MB/s
([#155](https://github.com/redox-rpi/redox-rpi/issues/155)).

This patch collects up to 8 records (1 MiB) before writing, as a program
writing 1 MiB at once would. Writes of 128 KiB and more, the 2 s timer and
the on-disk format are unchanged.

```bash
scripts/build-image.sh --experiment redoxfs-coalesce-1m
```

## Results

Pending: QEMU (`scripts/redoxfs-write-amp.py`, `scripts/sd-stress.py`), then
the Pi.
