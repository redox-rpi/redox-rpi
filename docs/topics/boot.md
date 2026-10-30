# Boot

## State

- Pi firmware → U-Boot → Redox bootloader (UEFI) → kernel, from the SD card
  or over the network (TFTP and HTTP from the build host, see
  [netboot.md](../netboot.md)).
- Network boot without an SD card, or with a card that has no
  `bootcode.bin`, tested on a Pi 3B+ on 2026-10-28.

## In progress

- [#18](https://github.com/redox-rpi/redox-rpi/issues/18) leave `pcid` and `audiod` out of the Pi images: passed on a
  Pi 3B+ as experiment `pi-clean-boot` (tests #14 and #5, 2026-10-28,
  keyboard checked in #5). Branch `clean-boot-default` makes it the
  default base patch `0030`: passes in QEMU and on the Pi (test #33,
  2026-10-29). Merged to main 2026-10-29 as base `0030`, ahead of sdhost (rc2 step 3).

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
  are 0, no rename. On branch `rpi-update` it keeps files open per handle
  (sequential writes ~370 KB/s in QEMU instead of slowing down with every
  MiB) and runs as a service at boot.
- Kernel and system updates without a new card (#15): `rpi-update` on
  branch `rpi-update` (experiment of the same name, needs `redox-fatfs2`
  and `rpi-fb`). The direct-boot FAT has two slots, `kernel8.img` +
  `initramfs` and `kernel8-b.img` + `initramfs-b`; the initramfs is the
  whole system. `rpi-update fetch` downloads a build from
  `http://192.168.178.120:8080/update` (written by
  `scripts/publish-rpi-update.sh`) into the free slot and checks
  `SHA256SUMS`; `rpi-update try` sets the firmware's tryboot flag (mailbox
  tag `0x00038064` through `/scheme/rpi-firmware/property`) and reboots.
  The firmware then reads `tryboot.txt` once, which starts the new slot
  with `REDOX_RPI_TRYBOOT=1` on the command line; a boot service commits
  (writes the slot into `config.txt`) when it sees that. If the new kernel
  does not come up, the next reset starts the old slot from `config.txt`.
  QEMU (2026-10-29): fetch, tryboot start (played with `--append`), commit
  and the normal start afterwards work; see the experiment's README. Only
  for the direct-boot card: the U-Boot card has one kernel in RedoxFS and
  no fallback. Plain HTTP and checksums only, home network.
- Watchdog for the fallback (#15, branch `rpi-update`): in a tryboot start
  rpi-boot-stub arms the BCM2837 watchdog (about 16 s), and
  `rpi-update watchdog`, an early boot service, restarts it until commit,
  for at most 2 minutes. A hanging kernel, a system that never gets to the
  commit service, or a failed commit then restart into the old slot
  without a power cycle. QEMU (2026-10-30) resets as soon as the watchdog
  is armed, so it tests the logic with `REDOX_RPI_WATCHDOG=dry`: commit
  stops it, without commit it stops after 120 s, a normal start leaves it
  alone.
- Packages ([#44](https://github.com/redox-rpi/redox-rpi/issues/44), v0.5.0): `pkg` with our
  own repository at `http://homenodellm:8080/pkg` (branch `pkg-repo`,
  experiments `pkg-hold` and `pkg-repo`). The hold list keeps our patched
  packages (kernel, relibc, base, …) on our builds (Pi test #52 passed);
  pkgutils patch 0003 makes `pkg update` replace files at all (Pi test #73
  passed; [PR #66](https://github.com/redox-rpi/redox-rpi/pull/66) for the hold list, [PR #75](https://github.com/redox-rpi/redox-rpi/pull/75) for
  the repository and 0003, branch `pkg-repo-tested`), patch 0004 compares each package against the first repository
  that has it, so upstream packages are updated too (QEMU 2026-10-30).
  Publish every build that goes onto Pis: `pkg update` compares
  checksums, so an older repository wins over a newer image.

## Next

- Pi test #72 of `rpi-update` after #18: does the Pi firmware start
  `tryboot.txt` once after the flag, and `config.txt` again after the
  watchdog reset? Needs a person to put in the direct-boot card.
