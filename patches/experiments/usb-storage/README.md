# USB sticks (experiment `usb-storage`)

[#25](https://github.com/redox-rpi/redox-rpi/issues/25). Applies after the
hotplug patches, base `0031`–`0033` on branch `usb-hotplug` (PR #68):
`--experiment usb-storage`.

- **base `0038`** (was `0033` before hotplug became base `0031`–`0033`): `dwc2d` serves a USB stick (bulk-only transport, SCSI
  commands) as the disk scheme `disk.usb-dwc2+<ports>-scsi`, named like
  upstream's `usbscsid`. `<ports>` is the hub ports from the root, e.g.
  `1.3`, or `0` for a stick on the root port. `0` is the whole stick,
  `0p0`, `0p1`, … its partitions (MBR or GPT, read by driver-block).
  - Whole blocks at any block offset; `fstat` gives the block size.
  - A stick plugged in after boot is found by the hotplug rescan; when it
    is unplugged the scheme goes away, and a stick that stops answering
    returns `ENODEV`.
  - One stick at a time. A full-speed stick behind the Pi's high-speed
    hubs is not supported (no split bulk transfers); sticks are
    high speed.

Mounting is the boot topic's side: `fatfsd /scheme/disk.usb-dwc2+…/0p0 usb0
&` for FAT, `redoxfs` for RedoxFS.

## Tests

- **QEMU** (`scripts/test-usb-storage.sh`, 2026-10-29, build
  `v0.3.0-dev-179-g01aa352+x7d64`): a 16 MiB stick plugged in after boot
  is found, its first block read, block 0 copied to block 2048 (seen in the
  image on the host), unplugged (the disk can no longer be opened), and
  plugged into the same port again (same scheme name, the copy reads back).
- driver-block opens a disk for writing once at a time: `dd if=X of=X` on
  the same disk fails with `ENOLCK`; go through a file.
- A scheme's name stays in the namespace after its socket is closed, so
  dwc2d unlinks it from the namespace on unplug; without that a stick
  plugged into the same port again could not register.
- Not yet on the Pi.
