# Collect up to 1 MiB of small writes (redoxfs, was experiment `redoxfs-coalesce-1m`)

**Default with PR #73:** the patch is `patches/redoxfs/0040`, after
`0037`–`0039`. `--experiment redoxfs-coalesce-1m` is still accepted and does
nothing.

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

Pi 3B+, test 230 (2026-11-04, A/B as test 175, report on main): all 18
sha256 match, no transfer error, `pi-test.py` 0 of 300 starts failed on both.
4 MiB written in 4 KiB calls: A 283 requests (10200 blocks, about 250 ms
card busy), B 86 requests (8620 blocks, about 110 ms), the same as a 1 MiB
write (77–78 requests, 8552 blocks).

| Call size | A (dd MB/s) | B (dd MB/s) |
|---|---|---|
| 4 KiB | 1.9–3.7 | 3.9–5.4 |
| 64 KiB | 1.1–5.5 | 5.1–8.8 |
| 1 MiB | unchanged | unchanged |
