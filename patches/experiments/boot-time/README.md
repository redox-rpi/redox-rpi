# Experiment: boot-time, shorter boot (#47)

Where the boot of the network-boot image spends its time, and cuts.

## Timeline

QEMU `raspi3b`, stable image `v0.3.0-dev-15-g24fbb96+x9f76`, from the SD
card (QEMU has no network boot), time-stamped with
[`boot-timeline.py`](boot-timeline.py):

| Time (s) | Step |
|---|---|
| 0.2 | U-Boot starts |
| 1.5–3.9 | U-Boot looks for the SD card and USB |
| 3.9–5.9 | U-Boot autoboot countdown, 2 s |
| 6.4 | Redox bootloader |
| 7.4 | mode selection: "Continuing with the selected mode in 5 seconds" (the QEMU script presses Enter at once; a Pi waits the 5 s) |
| 8.2–15.2 | bootloader loads kernel and initfs (21 MiB) from the SD card |
| 15.3 | kernel |
| 16.1 | initfs |
| 20.4 | root file system |
| 21–32 | `netstack` waits 11 s for a network adapter (QEMU without network card only) |
| 37.5 | `redox login:` |

### Inside Redox: which unit waits

init starts one unit at a time and waits until it is ready (`notify`,
`scheme` and `oneshot` units). Base patch
[`0025`](base/0025-init-report-units-that-take-100-ms-or-more-to-start.patch)
prints each unit that takes 100 ms or more. QEMU, no network card,
`v0.3.0-dev-213-g4e63067+x19b9` (times in s since init's first unit; QEMU
varies by a second or more between runs):

| Since init (s) | Unit | Took (ms) | What it waits for |
|---|---|---|---|
| 2.8 | `40_dwc2d.service` | 1600 | USB: enumeration before it reports ready, so `netstack` finds the adapter (on the Pi this includes the LAN7800, asked its owner) |
| 3.2 | `40_bcm2835-sdhcid.service` | 340 | SD card |
| 3.5 | `40_hwd.service` | 330 | devicetree drivers |
| 3.8 | `40_pcid-spawner-initfs.service` | 240 | nothing on the Pi (gone with clean boot, #18) |
| 4.7 | `50_rootfs.service` | 920 | disk: RedoxFS mount |
| 5.5 | `00_pcid-spawner.service` | 510 | nothing on the Pi (#18) |
| 5.7–12.0 | `00_tmp` (script, not timed) | ~6000 | disk: `rm` and `mkdir`, each the 10 MB coreutils program read from the disk |
| 12.4 | `10_smolnetd.service` (netstack) | 420 | network stack; the login prompt comes after it |
| 13.3 | `fbcond@2.service` | 880 | screen console |
| 14.6 | `20_audiod.service` | 500 | nothing on the Pi (#18) |
| 15.1 | `getty@2.service` | 500 | console login |

The first programs started from the root file system pay for reading
their binaries and libraries from the disk: QEMU's SD card reads about
3 MiB/s. On the Pi's network boot the disk is in RAM, so the Pi numbers
(#53) show how much is the disk.

The real Pi boots over the network: firmware over TFTP, then U-Boot runs
`boot.scr`, which starts USB, asks DHCP and loads `netboot.scr`; that
started USB and asked DHCP again before it loaded the image over HTTP. Its
timeline is the Pi test.

## Cuts

- Bootloader patch
  [`0003`](bootloader/0003-Wait-1-second-instead-of-5-for-a-key-at-the-mode-sel.patch):
  the mode selection waits 1 s instead of 5 for a key (our own
  `bootloader/0001` made it 5). A key within the second still switches to
  the normal selection; `e` and `l` still work in that second.
- `config/netboot.cmd` (not an experiment, on branch `boot-time`): no second
  `usb start` and `dhcp` when `boot.scr` already has an address.
- Base patch
  [`0026`](base/0026-init-clear-and-recreate-tmp-itself-instead-of-with-r.patch):
  init clears and recreates `/tmp` (mode 1777) itself; the `00_tmp` script
  no longer starts `rm` and `mkdir`. QEMU, `v0.3.0-dev-214-g67fdc06+x19b9`:
  root file system to `redox login:` 9.8 s instead of 11.3 s; the next
  program (`sudo`) now pays part of the disk reads (2.6 s), so the gain on
  a fast disk is smaller. `/tmp` is `drwxrwxrwt` and writable.
- With clean boot (#18, merged last for v0.3.0) `pcid-spawner` twice and
  `audiod` go away: about 1.2 s in QEMU.
- Not changed here: U-Boot's 2 s autoboot countdown. It is in the shared
  `u-boot.bin` (`scripts/build-uboot.sh`), used by every network boot, and
  takes a key on the serial console to stop U-Boot. Proposal: 1 s, after
  the Pi timeline.

```bash
scripts/build-image.sh --experiment timesync --experiment rtl8152 --experiment boot-time
```

## Results

See [test-reports.md](../../../docs/test-reports.md).
