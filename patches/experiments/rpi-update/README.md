# Experiment: rpi-update, a new kernel and system without a new card

For the direct firmware boot ([#15](https://github.com/redox-rpi/redox-rpi/issues/15),
`make-sdcard.sh --direct`). `rpi-update` ([`tools/rpi-update`](../../../tools/rpi-update))
fetches a newer build from the build host into a second slot on the FAT
partition and starts it once with the firmware's tryboot. If the new system
comes up, it stays; if not, the watchdog (or the next reset) starts the
old one.

```bash
scripts/build-image.sh --experiment redox-fatfs2 --experiment rpi-fb --experiment rpi-update
scripts/make-sdcard.sh --direct upstream/redox/build/aarch64/minimal/harddrive.img
# later, after a newer build:
scripts/publish-rpi-update.sh        # to /srv/tftp/update
```

On the Pi:

```bash
rpi-update status    # slots, running slot, this build and the server's
rpi-update fetch     # into the free slot, checks SHA256SUMS
rpi-update try       # tryboot flag, reboot
# the new system commits itself at boot (90_rpi-update-commit.service)
```

`rpi-fb` is needed for `try`: it adds `/scheme/rpi-firmware/property`
(patch 0024), through which `rpi-update` sets the tryboot flag.

## Layout

The FAT partition has room for two slots (make-sdcard.sh sizes it):

| Slot | Kernel | System |
| --- | --- | --- |
| a | `kernel8.img` | `initramfs` |
| b | `kernel8-b.img` | `initramfs-b` |

The initramfs is the whole root filesystem (it runs from RAM), so a slot
is kernel and system together. `config.txt` names the slot that starts.

1. `fetch` writes the free slot, checks both files against `SHA256SUMS`,
   then writes `tryboot.txt` (config.txt with the new slot and
   `cmdline=cmdline-try.txt`) and `cmdline-try.txt` (the command line plus
   `REDOX_RPI_TRYBOOT=1`).
2. `try` sends mailbox tag `0x00038064` (SET_REBOOT_FLAGS) with 1 and
   reboots. The firmware reads `tryboot.txt` instead of `config.txt`, once.
3. rpi-boot-stub puts `REDOX_RPI_TRYBOOT=1` into the environment. When init
   reaches `90_rpi-update-commit.service`, `rpi-update commit` sees it in
   `/scheme/sys/env`, writes the new slot into `config.txt` and deletes
   `tryboot.txt`.
4. If the new kernel hangs or the system does not get that far, the
   watchdog resets the board (see below) and the firmware starts
   `config.txt` again: the old slot. A power cycle does the same.

## Watchdog

The BCM2837's watchdog (power management block at `0x3f100000`, the one
the kernel resets through) counts down from at most about 16 s and then
resets the board.

- rpi-boot-stub arms it with that maximum in a tryboot start (command line
  `REDOX_RPI_TRYBOOT=1`), just before it starts the kernel, and adds
  `RPI_WATCHDOG_ARMED=1` to the environment.
- `05_rpi-update-watchdog.service` (`rpi-update watchdog`, about 3 s after
  the kernel starts) restarts the countdown every 4 s through
  `/scheme/memory/physical@dev`.
- `rpi-update commit` stops the watchdog and writes
  `/tmp/rpi-update-committed`; the loop sees it and ends.
- Without a commit within 2 minutes the loop stops restarting it, and
  about 16 s later the old slot starts.

So a kernel that hangs before userspace, a system that never gets to
service 90, or a commit that fails all end in the old slot without a
person. `REDOX_RPI_WATCHDOG=off` on the command line leaves the watchdog
out; `=dry` also lets `rpi-update` only log the register writes.

## QEMU test (2026-10-29)

Image X v0.3.0-dev-208-g12e108b+xe113 on a `make-sdcard.sh --direct` card;
Y is the same build with `-y` added to its BUILD_ID, published with
`publish-rpi-update.sh`. QEMU has no Pi firmware, so the tryboot start is
played by booting slot b's files with `qemu-raspi3b.sh --direct <dir>
--append REDOX_RPI_TRYBOOT=1`.

1. Slot a: `status` sees the server build; `fetch` writes `kernel8-b.img`
   and the 245 MiB `initramfs-b` (about 11 minutes, ~370 KB/s through
   fatfsd), both match `SHA256SUMS` on the host. `try` reboots; QEMU
   ignores the flag, so slot a comes back and the service says "not a
   tryboot start". `config.txt` still names slot a: the fallback state.
2. Tryboot start of slot b: `REDOX_RPI_TRYBOOT=1` in `/scheme/sys/env`,
   BUILD_ID `…-y`, the service prints "slot b committed", `config.txt`
   names slot b, `tryboot.txt` and `cmdline-try.txt` are gone.
3. Normal start of that card: slot b from `config.txt`, nothing to commit.

Found on the way: one write per TCP segment made fatfsd walk the cluster
chain for every 1.5 KiB (11 KB/s, slower with every MiB). Fixed in both:
rpi-update writes whole MiB, fatfsd keeps files open per handle.

The firmware side (does the Pi start `tryboot.txt` once after the flag,
and `config.txt` again after a reset) can only be tested on a Pi.

## QEMU test of the watchdog (2026-10-30)

Image v0.3.0-dev-211-g80cc863+xe113 (`redox-fatfs2`, `rpi-fb`,
`rpi-update`) on a `make-sdcard.sh --direct` card.

- `--append REDOX_RPI_TRYBOOT=1`: QEMU resets as soon as the stub arms the
  watchdog (its model has no countdown), 15477 starts in 400 s. That shows
  the write reaches the PM block, but QEMU cannot test the timing, so
  the next runs use `REDOX_RPI_WATCHDOG=dry`.
- Tryboot start with a `tryboot.txt` on the card: `watchdog: restarting it
  until commit`, one restart (`0x24 <- 0x5a0fffff`, `0x1c <- 0x5a000020`),
  `slot a committed`, `0x1c <- 0x5a000102` (stop), `watchdog: committed
  after 4 s, stopped`.
- The same with a system whose commit service only prints the status: 30
  restarts, then `watchdog: no commit after 120 s, the old slot starts in
  about 16 s`, and no more writes.
- Normal start: `watchdog not armed by rpi-boot-stub, nothing to do`, `not
  a tryboot start, nothing to commit`.

## Services

- `05_rpi-update-watchdog.service`: `rpi-update watchdog`, does nothing
  outside a tryboot start.
- `66_boot-fat.service`: `fatfsd /scheme/disk.mmc/0p0 boot`.
- `90_rpi-update-commit.service`: `rpi-update commit`, does nothing
  outside a tryboot start.
- `/etc/rpi-update.conf`: `url=http://192.168.178.120:8080/update`.

## Limits

- Only the direct-boot card. The U-Boot card (`make-sdcard.sh --local`)
  has one kernel in RedoxFS and no fallback.
- Plain HTTP, no signature: home network only. The checksums only catch
  broken downloads.
- "Came up" means init got to service 90. A system that boots but has no
  network is still committed.
- The watchdog's real countdown, and whether the Pi firmware starts
  `config.txt` after a watchdog reset, are only tested on a Pi (test #72).
- The 2 minutes are fixed; a system that is slow to reach service 90
  falls back although it works.
