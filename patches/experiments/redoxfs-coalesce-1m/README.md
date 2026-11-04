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

QEMU raspi3b, 2026-11-04. A is PR #73 (`v0.4.0-dev-157-g71d7ba7d+x08c5`),
B is A plus this patch (`v0.4.0-dev-158-gb1e99de3+xcad5`), both with
`sdhost-stats`.

`scripts/redoxfs-write-amp.py` (1 MiB per run, three runs each, with
`-usb -netdev user,id=n0 -device usb-net,netdev=n0`; without a USB network
adapter the boot stops after `dwc2d`), bytes the disk saw per byte written:

| Call size | A | B |
|---|---|---|
| 4 KiB | 1.3 (1351680 bytes) | 1.1–1.2 (1146880–1306624) |
| 64 KiB | 1.3 (1413120) | 1.2 (1208320) |
| 1 MiB | 1.1 (1114112) | 1.1 (1114112) |

QEMU's SD card traces 512-byte blocks, not commands, so the number of
requests needs the Pi's `sdhost-stats`.

`scripts/sd-stress.py --big-mib 32 --small 300 --cycles 0` in
`/home/root/stress`: pass on both (0 missing, 0 changed).

Pi: test 230 queued (A/B as test 175).
