# Experiment pi400: first test version on the Pi 400 (BCM2711)

Work package #299, plan in [docs/topics/pi400.md](../../../docs/topics/pi400.md).
Ran on a real Pi 400 on 2026-11-10: network boot to the login prompt.

- `kernel/0001`: the GIC-400 of the BCM2711 names itself as its interrupt
  parent in the devicetree; without the fix no interrupt controller is
  initialized (no serial interrupt, one CPU only).
- `base/0001`: `dwc2d`, `bcm2835-sdhostd` and `bcm2835-pwmaudiod` exit on
  the BCM2711, where the card is on EMMC2, USB is behind PCIe and the PWM
  audio block is not set up yet.
- `base/0002`: `bcmgenetd`, the Ethernet port (GENET v5 with the BCM54213PE
  PHY) as `network.genet` (step e, #306), and a `mac` line (firmware
  mailbox) in rpi-firmwared's board file. Not tested on the Pi 400 yet.
- The external-abort fix (#311) is on main as `patches/kernel/0051`.

The root filesystem runs from RAM (`bootloader-live.efi`), as with network
boot, because there is no EMMC2 driver yet (step f, #307).

## QEMU

`raspi4b` in QEMU 10.2 (homenodellm): no PCIe, no GENET.
