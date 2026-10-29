# Experiment: rpi-fb, display through the Pi firmware (#33)

Step 1 of the [graphics topic](../../../docs/topics/graphics.md): a display
driver that gets its framebuffer from the Pi firmware itself, instead of
only inheriting the boot framebuffer.

```bash
scripts/build-image.sh --experiment timesync --experiment rtl8152 --experiment rpi-fb
```

- [`0023`](base/0023-rpi-firmwared-raw-property-file-for-other-drivers.patch):
  `rpi-firmwared` gets a `property` file (root only, not listed): a driver
  writes a whole property message, reads back the firmware's answer. So
  only `rpi-firmwared` uses the mailbox.
- [`0024`](base/0024-rpi-fbd-Raspberry-Pi-display-through-the-firmware.patch):
  the new driver `rpi-fbd` (initfs, after `rpi-firmwared`).
