# USB sticks with FAT (experiment `usb-fat`)

[#25](https://github.com/redox-rpi/redox-rpi/issues/25), the filesystem
side. `dwc2d` serves a stick as the disk scheme `disk.usb-dwc2+<ports>-scsi`
(experiment `usb-storage`, the USB topic's side); this experiment mounts
its FAT partitions. Applies after `usb-hotplug` and `usb-storage`:

```bash
scripts/build-image.sh --experiment usb-hotplug --experiment usb-storage --experiment usb-fat
```

## What it adds

- **`usbmountd`** (`tools/usbmountd`, started at boot by
  `70_usbmountd.service`): once a second it looks at `/scheme` for
  `disk.usb-*` schemes. For each disk in one (`0`, and `1`, `2`, … for the
  slots of a card reader) it reads the first block of each partition
  (`0p0`, `0p1`, …, or the whole disk without a partition table) and starts
  `fatfsd <partition> usbN` for FAT12/16/32, with the lowest free N. exFAT
  (most sticks over 32 GB) is reported and left alone. A disk is mounted
  once it shows the same partitions in two rounds in a row.
- **`usbmountd eject usb0`**: the safe unmount. fatfsd writes what is open,
  marks the volume clean and ends; `eject` waits until `/scheme/usb0` is
  gone and says `the stick can be removed`. The stick is mounted again only
  after unplugging it and plugging it in again. `usbmountd list` shows the
  mounted sticks.
- Unplugged without `eject`: usbmountd sees the disk go away and stops its
  fatfsd, so `/scheme/usb0` goes away and the name is free again. Data
  written before is on the stick (fatfsd writes through), but the volume
  stays marked dirty, as on other systems.
- **fatfsd** (`tools/redox-fatfs2`, from branch `fw-direct-boot`, #15):
  - `rename` (`mv`), also over an existing file or empty directory, and
    into another directory;
  - SIGTERM unmounts (fatfs' unmount: FAT32 free-space info and the clean
    flag);
  - removes its scheme name from the namespace when it ends, also when the
    disk goes away, so a stick plugged in again gets the name again;
  - an inode number per path in `fstat` and `getdents`: `mv a b` compared
    them and refused with "are the same file" while they were all 0.

## Tests

QEMU raspi3b, `scripts/test-usb-fat.sh`, 2026-10-30, build
`v0.3.0-dev-194-ge48e256+xa7bd`, all 16 checks pass. A 64 MiB stick with an
MBR and a FAT32 partition made on the host with mtools, plugged in after
boot through QEMU's monitor:

- mounted as `/scheme/usb0`; a file from the host reads back;
- 1 MiB written in 4 KiB blocks, `mkdir`, `mv` over an existing file, `mv`
  into a subdirectory, `rm`;
- `usbmountd eject usb0`: `/scheme/usb0` gone, then unplugged;
- on the host: the volume is marked clean, the moved file has the SHA-256
  the guest computed, the replaced and removed files are right, the host's
  own file untouched;
- plugged in again: mounted again as `usb0`, a 1 MiB file the host put on
  it reads back with the right SHA-256;
- unplugged without eject: fatfsd ends, `/scheme/usb0` is gone.

On the Pi: test 100 (needs a person with a stick).
