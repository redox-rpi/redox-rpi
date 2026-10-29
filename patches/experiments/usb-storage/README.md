# USB sticks (experiment `usb-storage`)

[#25](https://github.com/redox-rpi/redox-rpi/issues/25). Applies after the
experiment `usb-hotplug` (`--experiment usb-hotplug --experiment
usb-storage`).

- **base `0033`:** `dwc2d` serves a USB stick (bulk-only transport, SCSI
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
