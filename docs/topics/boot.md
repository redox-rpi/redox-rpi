# Boot

## State

- Pi firmware → U-Boot → Redox bootloader (UEFI) → kernel, from the SD card
  or over the network (TFTP and HTTP from the build host, see
  [netboot.md](../netboot.md)).
- Network boot without an SD card, or with a card that has no
  `bootcode.bin`, tested on a Pi 3B+ on 2026-10-28.

## In progress

- [#18](https://github.com/redox-rpi/redox-rpi/issues/18) leave `pcid` and `audiod` out of the Pi images: experiment
  [`pi-clean-boot`](../../patches/experiments/pi-clean-boot/README.md)
  on branch `pi-clean-boot`. init units get `condition_board_not`; the
  two `pcid-spawner` units and `audiod` do not start on `raspi3bp`, `hwd`
  does not start `pcid`, and `rtcd` runs on x86 only. On a Pi 3B+ (test
  #14, 2026-10-28) the boot log has none of their errors, the network
  comes up and `scripts/pi-test.py` passes with 0 of 300 failed starts.
  Open: typing on the USB keyboard, then the merge to `main`.

- [#15](https://github.com/redox-rpi/redox-rpi/issues/15) boot the kernel straight from the Pi firmware, without
  U-Boot and the Redox bootloader (milestone v0.4.0), branch
  `fw-direct-boot`: `tools/rpi-boot-stub` as `kernel8.img` wraps the
  unchanged kernel; boots to the login prompt in QEMU `raspi3b`
  (`scripts/fw-direct-boot.sh --test`). `NAME=value` words of the
  command line (`cmdline.txt`) go into the environment. The MAC address
  probably comes from the firmware's devicetree; to check at the first Pi
  test with the serial console. `make-sdcard.sh --direct` writes a card
  for it (checked in QEMU); Pi test #18 in [test-plan.md](../test-plan.md).

## Next

- FAT boot partition from Redox, for kernel updates and `config.txt`
  (#15): `redox-fatfs` builds but uses the old scheme interface and does not
  run; port it or write a small daemon on the `fatfs` crate (decision open,
  see the issue).
