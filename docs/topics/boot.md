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
  does not start `pcid`, and `rtcd` runs on x86 only. In QEMU the boot log
  has none of their errors, the network comes up and 40 of 40 starts
  pass. Waits for the Pi test (#14 in the
  [test plan](../test-plan.md)).

## Next

- [#15](https://github.com/redox-rpi/redox-rpi/issues/15) boot the kernel straight from the Pi firmware, without
  U-Boot.
