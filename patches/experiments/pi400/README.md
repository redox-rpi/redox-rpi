# Experiment pi400: first test version on the Pi 400 (BCM2711)

Work package #299, plan in [docs/topics/pi400.md](../../../docs/topics/pi400.md).
Ran on a real Pi 400 on 2026-11-10: network boot to the login prompt.

- `kernel/0001`: the GIC-400 of the BCM2711 names itself as its interrupt
  parent in the devicetree; without the fix no interrupt controller is
  initialized (no serial interrupt, one CPU only).
- `base/0001`: `dwc2d`, `bcm2835-sdhostd` and `bcm2835-pwmaudiod` exit on
  the BCM2711, where the card is on EMMC2, USB is behind PCIe and the PWM
  audio block is not set up yet.
- The external-abort fix (#311) is on main as `patches/kernel/0051`.
- SD card slot through EMMC2 (#307):
  - `base/0003`: `bcm2835-sdhcid` drives EMMC2; on the Pi 3 it exits and
    `bcm2835-sdhostd` keeps the card.
  - `base/0004`, `0017`: `rpi-firmwared` power-cycles the card slot at
    3.3 V on the Pi 4 and 400 (GPIO expander pins 6 and 4, set as outputs
    first), as Linux does; nothing else in Redox powers the slot.
  - `base/0005`, `0006`: `rpi-firmwared` lists the EMMC2 clock and
    `bcm2835-sdhcid` divides the SD clock from it.
  - `base/0007`: no `disk.mmc` disk without a usable card.
  - `base/0008`, `0009`: card init retries with a longer slot power cycle;
    ACMD41 with HCS and 3.3 V only, polled for up to 2 s.

The root filesystem runs from RAM (`bootloader-live.efi`), as with network
boot, because there is no EMMC2 driver yet (step f, #307).

## QEMU

`raspi4b` in QEMU 10.2 (the build server): no PCIe, no GENET.

QEMU's own SD card sits on the Arasan controller, not on EMMC2.
`scripts/qemu-raspi4b.sh --emmc2 <image>` attaches a second card to EMMC2,
so `bcm2835-sdhcid` can be tested with a card, for example the #307 test
card: `/scheme/data` is then mounted from its second partition.
