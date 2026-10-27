# Experiment: bcm2835-sdhostd, a driver for the sdhost SD controller

New driver for the BCM2835 sdhost controller (`brcm,bcm2835-sdhost`), which
replaces `bcm2835-sdhcid` (Arasan controller) in the initfs services. It
routes GPIO 48–53 to the controller, initializes the card and serves
`disk.mmc`.

```bash
scripts/build-image.sh --experiment sdhost-driver
```

**Result:** works in QEMU with SDSC and SDHC cards (4-bit bus, 25 MHz).
On the real Pi 3B+, a netboot image with it (2026-10-27, commit `2c66e72`)
did not come back on the network after a reboot; the cause is unknown, no
serial console was attached. Needs a test with the serial console.

The clock divider assumes a 400 MHz core clock; at 250 MHz the card only
runs slower.
