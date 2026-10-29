# Boot

## State

- Pi firmware → U-Boot → Redox bootloader (UEFI) → kernel, from the SD card
  or over the network (TFTP and HTTP from the build host, see
  [netboot.md](../netboot.md)).
- Network boot without an SD card, or with a card that has no
  `bootcode.bin`, tested on a Pi 3B+ on 2026-10-28.

## In progress

- [#47](https://github.com/redox-rpi/redox-rpi/issues/47) boot time (v0.4.0), branch `boot-time`: QEMU timeline 37.5 s to
  `redox login:`; the bootloader waits 5 s at the mode selection and U-Boot
  counts down 2 s on every boot. Experiment
  [`boot-time`](../../patches/experiments/boot-time/) waits 1 s in the
  bootloader, `config/netboot.cmd` no longer asks DHCP twice, init recreates
  `/tmp` itself instead of starting `rm` and `mkdir`, and init reports slow
  units. Pi (#53): 33.7 s → 29.3 s from reset to `redox login:`. Left:
  `dwc2d` 6.2 s (about 1.2 s per USB hub and 1.7 s for the network link,
  asked its owner, who drops the hub waits in `dwc2d-boottime`), U-Boot's
  2 s countdown, 3.8 s for the image over HTTP. New: init starts `notify`
  services in parallel and the root file system no longer waits for USB
  (base `0027`): QEMU init to login 6.0 s instead of 14.9 s. Pi test
  queued.

- [#18](https://github.com/redox-rpi/redox-rpi/issues/18) leave `pcid` and `audiod` out of the Pi images: experiment
  `pi-clean-boot` on branch `pi-clean-boot` passes on a Pi 3B+ (test #14,
  2026-10-28): no errors from these services, network up, `pi-test.py`
  passes. Open: typing on the USB keyboard, then the merge.

- [#15](https://github.com/redox-rpi/redox-rpi/issues/15) boot the kernel straight from the Pi firmware, without
  U-Boot and the Redox bootloader (milestone v0.4.0), branch
  `fw-direct-boot`: `tools/rpi-boot-stub` as `kernel8.img` wraps the
  unchanged kernel; boots to the login prompt in QEMU `raspi3b`
  (`scripts/fw-direct-boot.sh --test`). `NAME=value` words of the
  command line (`cmdline.txt`) go into the environment. The MAC address
  probably comes from the firmware's devicetree; to check at the first Pi
  test with the serial console. `make-sdcard.sh --direct` writes a card
  for it (checked in QEMU); Pi test #18 in [test-plan.md](../test-plan.md).
- FAT boot partition from Redox (#15): `redox-fatfs2` on branch
  `fw-direct-boot` replaces the unmaintained `redox-fatfs`. Its daemon
  `fatfsd` serves a FAT partition as a scheme, built on the `fatfs` crate
  (MIT), e.g. `fatfsd /scheme/disk.mmc/0p0 boot`. In QEMU (2026-10-28) it
  lists, reads, writes, creates directories and removes files on the
  direct-boot card; the host sees the changes with mtools. Open: file dates
  are 0, no rename, no service yet.

## Next

- `fatfsd` at boot: a service that mounts the boot partition, once the Pi
  test #18 shows the SD card's scheme name; then kernel and `config.txt`
  updates from Redox (#15).
