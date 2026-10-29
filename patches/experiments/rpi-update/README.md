# Experiment: rpi-update, a new kernel and system without a new card

For the direct firmware boot ([#15](https://github.com/redox-rpi/redox-rpi/issues/15),
`make-sdcard.sh --direct`). `rpi-update` ([`tools/rpi-update`](../../../tools/rpi-update))
fetches a newer build from the build host into a second slot on the FAT
partition and starts it once with the firmware's tryboot. If the new system
comes up, it stays; if not, the next reset starts the old one.

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
4. If the new kernel hangs or the system does not get that far, a power
   cycle (or the watchdog, once there is one) starts `config.txt` again:
   the old slot.

## Services

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
- No watchdog yet: a hanging new kernel needs a power cycle to fall back.
