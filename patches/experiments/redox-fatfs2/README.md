# Experiment: redox-fatfs2, the FAT boot partition from Redox

With the direct firmware boot ([#15](https://github.com/redox-rpi/redox-rpi/issues/15),
`make-sdcard.sh --direct`), the boot files `kernel8.img`, `initramfs` and
`config.txt` are on the FAT partition of the SD card. Redox has no working
FAT driver today: `redox-fatfs` is unmaintained (last change 2020), uses the
old scheme interface and does not start. `redox-fatfs2`
([`tools/redox-fatfs2`](../../../tools/redox-fatfs2)) replaces it: a small
new daemon, `fatfsd`, built on the maintained `fatfs` crate. It serves a
FAT partition as a scheme, so Redox can read and write it, for example to
update the boot files.

```bash
scripts/build-image.sh --experiment redox-fatfs2
```

The experiment has no patches, only a [`config.toml`](config.toml) that adds
the `redox-fatfs2` package (binary `fatfsd`). There is no service yet,
start it by hand:

```bash
fatfsd /scheme/disk.mmc/0p0 boot &    # first partition of the SD card
ls /scheme/boot
cat /scheme/boot/config.txt
echo test > /scheme/boot/test.txt
```

## How it works

- The FAT code is the [`fatfs`](https://github.com/rafalh/rust-fatfs) crate,
  0.3.6. fatfsd only binds it to today's scheme interface (`redox-scheme`
  0.12, `SchemeSync`, as in redoxfs).
- Each open file or directory is a path. Every read or write opens the file
  in the crate again, so no crate object lives across requests.
- The disk is read and written in whole blocks of the device's block size
  (`st_blksize` from driver-block: 512 bytes on SD cards and most USB
  sticks, 4096 on some).
- When the disk goes away (`ENODEV` or `ENXIO`, e.g. an unplugged USB
  stick), fatfsd exits and its scheme disappears.
- Any Redox disk scheme works, e.g. a USB stick from `usbscsid` as
  `fatfsd /scheme/disk.usb-<…>-scsi/0p0 usb0` ([#25](https://github.com/redox-rpi/redox-rpi/issues/25)).
- Supported: open, create, read, write, truncate, `stat`, directory
  listing, unlink and rmdir. Not supported: rename, timestamps (the crate's
  `chrono` feature is off), permissions (FAT has none, files show as 0644,
  directories as 0755).
- FAT is case-insensitive; lookups ignore the case of ASCII letters.

## Licences

No GPL code. fatfsd itself is MIT, like the other tools.

| Crate | Licence |
| --- | --- |
| `fatfs` 0.3.6 | MIT |
| `byteorder` (used by `fatfs`) | MIT or Unlicense |
| `bitflags` 1 (used by `fatfs`) | MIT or Apache-2.0 |
| `log` (used by `fatfs`) | MIT or Apache-2.0 |
| `redox-scheme`, `libredox`, `redox_syscall` | MIT |
| `libc`, `plain`, `bitflags` 2 (used by the Redox crates) | MIT or Apache-2.0 |

## Open

- The FAT dirty flag: `fatfs` sets it on the first write and clears it
  only at unmount, and fatfsd never unmounts. Linux may report the
  partition as not cleanly unmounted (`fsck.vfat` fixes it).
- One fatfsd per partition. Two programs writing the same partition, for
  example fatfsd and a second mount, break the filesystem.
- A service that mounts the boot partition at start, once the scheme name
  of the SD card is fixed on the Pi.
