# Experiment: grow the root filesystem to the whole SD card

Issue [#24](https://github.com/redox-rpi/redox-rpi/issues/24). For the SD card
image (`--sdcard`, `scripts/make-sdcard.sh --local`): a small image grows to
the whole card on the first boot.

`growfs` runs in the initfs before the root filesystem is mounted
(`45_growfs.service`, before `50_rootfs`), on every boot, and compares sizes
only:

1. If the last MBR partition holds the root RedoxFS (the bootloader's
   `REDOXFS_UUID`) and ends at least 64 MiB before the end of the card, it
   extends that partition to the end of the card and restarts. The disk driver
   reads the partition table only when it starts.
2. If the RedoxFS is at least 64 MiB smaller than its partition, it grows it
   with `redoxfs-resize <partition> max` (now also in the initfs).

A network boot has its root elsewhere, so the uuid does not match and the card
is left alone.

```bash
scripts/build-image.sh --sdcard --experiment timesync --experiment grow-root
scripts/make-sdcard.sh --local upstream/redox/build/aarch64/sdcard/harddrive.img --small --out build/grow.img
```

`--small` shrinks the root RedoxFS to its contents (`redoxfs-resize min`) and
makes the root partition just as large: a 149 MiB image (23 MiB as `.xz`).

Needs the sdhost driver, which main has as base patches `0025`–`0028`.

## Results

QEMU `raspi3b`, `v0.3.0-dev-81-g5ceb150+x9922` (2026-10-28):

- 1.1 GB image (990 MiB partition) on a 2 GiB card: first boot extends the
  partition to 1984 MiB and restarts; the second boot grows the RedoxFS to
  1.94 GiB (84 MiB used) and logs in; a third boot changes nothing.
- `--small` image (85 MiB partition) on a 4 GiB card: partition to
  4032 MiB, RedoxFS to 3.94 GiB, then 200 MiB written with `conv=fsync`
  (3.1 MB/s) and hashed.
- First try: `redoxfs-resize` got `ENOLCK` because `growfs` still held the
  whole disk open; it closes it now.

On the Pi: test 29 in `docs/test-plan.md` (needs a person to swap the card).
