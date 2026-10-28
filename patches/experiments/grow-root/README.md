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
scripts/build-image.sh --sdcard --experiment timesync --experiment rtl8152 --experiment grow-root
scripts/make-sdcard.sh --local upstream/redox/build/aarch64/sdcard/harddrive.img --out build/grow.img
```

Needs the sdhost base patches (branch `sdhost-default`).

## Results

None yet.
