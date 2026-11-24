# Experiment pi400: first test version on the Pi 400 (BCM2711)

Work package #299, plan in [docs/topics/pi400.md](../../../docs/topics/pi400.md).
Ran on a real Pi 400 on 2026-11-10: network boot to the login prompt.

- `kernel/0001`: the GIC-400 of the BCM2711 names itself as its interrupt
  parent in the devicetree; without the fix no interrupt controller is
  initialized (no serial interrupt, one CPU only).
- `base/0001`: `dwc2d`, `bcm2835-sdhostd` and `bcm2835-pwmaudiod` exit on
  the BCM2711, where the card is on EMMC2, USB is behind PCIe and the PWM
  audio block is not set up yet.
- `base/0021`: a control transfer in `xhcid` times out after 5 s (as in
  Linux), stops endpoint 0 and returns `ETIMEDOUT`. Before, a mouse whose
  request never completed held up every other USB device, the built-in
  keyboard included, because `xhcid` serves control transfers one after
  the other (#710, pi-tests row 620). The trigger, `SET_IDLE` sent to a
  mouse that stalls it, is gone in `patches/base/0124`.
- The external-abort fix (#311) is on main as `patches/kernel/0051`.

The root filesystem runs from RAM (`bootloader-live.efi`), as with network
boot, because there is no EMMC2 driver yet (step f, #307).

## QEMU

`raspi4b` in QEMU 10.2 (the build server): no PCIe, no GENET.
