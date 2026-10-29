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
- Not changed here: U-Boot's 2 s autoboot countdown. It is in the shared
  `u-boot.bin` (`scripts/build-uboot.sh`), used by every network boot, and
  takes a key on the serial console to stop U-Boot. Proposal: 1 s, after
  the Pi timeline.

```bash
scripts/build-image.sh --experiment timesync --experiment rtl8152 --experiment boot-time
```

## Results

See [test-reports.md](../../../docs/test-reports.md).
