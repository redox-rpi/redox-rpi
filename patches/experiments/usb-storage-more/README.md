# Card readers and several sticks (experiment `usb-storage-more`)

Applies after the experiments `usb-hotplug` and `usb-storage`
(`--experiment usb-hotplug --experiment usb-storage --experiment
usb-storage-more`). Issue: [#52](https://github.com/redox-rpi/redox-rpi/issues/52),
follow-up of #25.

- **base `0038`:** `dwc2d` asks a stick or card reader for its logical
  units (GET MAX LUN) and sends every command to its LUN. Each LUN with a
  medium is a disk of the device's scheme under its number:
  `/scheme/disk.usb-dwc2+<ports>-scsi/<lun>`, so a stick stays `.../0` and
  a reader's second slot is `.../1`. An empty slot ("medium not present")
  is left out at once instead of after 5 s of waiting.
  - Several sticks and readers at the same time, each with its own scheme;
    they share host channels 4 and 5, one command at a time.
  - The log names the scheme when a device is removed:
    `dwc2d: mass storage /scheme/<name> removed`.
  - Not yet: a card put into a reader after it was plugged in (unplug and
    plug the reader in again); full-speed sticks behind the Pi 3B+'s
    high-speed hub (no split bulk transfers).

## Tests

- **QEMU** (`scripts/test-usb-storage.sh`): the stick checks of
  `usb-storage`, then a card reader (QEMU's `usb-bot`, plugged in with
  `attached=false` and `qom-set … attached true` so all LUNs are there
  first) with cards in LUN 0 (8 MiB) and LUN 2 (4 MiB) and an empty CD
  slot as LUN 1, next to the stick: both cards and the stick are read,
  the empty slot is left out, the reader is unplugged and the stick still
  works.
