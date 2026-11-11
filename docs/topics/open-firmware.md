# Open firmware

Replacing everything closed from Broadcom on the Pi, as far as possible
([#230](https://github.com/redox-rpi/redox-rpi/issues/230)). That covers
the boot firmware on the VPU (`bootcode.bin`, `start*.elf`), the ARM
stub, what `start.elf` does at run time (mailbox, display), and the blobs
of other chips such as Wi-Fi. Only the Boot ROM in the chip stays. We
start with the Pi 3B+. It lives in its
own repository,
[redox-rpi/rpi-open-firmware](https://github.com/redox-rpi/rpi-open-firmware)
(GPLv2+), together with all of its documentation:

- [Broadcom's firmware per Pi model](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/broadcom-firmware.md):
  what `bootcode.bin`, `start*.elf`, `fixup*.dat` and the `armstub*` do,
  and their variants on the Zero W, Zero 2 W, 3B+ and Pi 400.
- [Boot chain by release](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/boot-chain.md):
  how v0.4.0, v0.5.0 and the development card start, and the steps to no
  Broadcom code after the Boot ROM.
- [Our firmware](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/own-firmware.md),
  [Pi test results](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/test-results.md),
  [rpi-open-firmware (2019)](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/rpi-open-firmware-2019.md)
  and [lk-overlay](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/lk-overlay.md)
  on our Pi.

## State

- Every redox-rpi image still boots with Broadcom's firmware.
- On the test Pi only: the SD card's `bootcode.bin` is our boot selector.
  Enter on the serial console opens our monitor (our SDRAM set-up, code
  loaded over serial); otherwise Broadcom's `bootcode.bin` boots as before.
  Passed on the Pi (2026-11-11), so firmware tests need no card swap.

## In progress

- M2: our VPU code starts the ARM in AArch64 (rpi-open-firmware PR #7,
  Pi test row 332); our `armstub8` (PR #4).

## Next

- M2: load `armstub8`, U-Boot and the devicetree from the SD card.
- M3 mailbox service, M4 HDMI, M5 network boot and 3D.
- Decision on a project of its own: [#333](https://github.com/redox-rpi/redox-rpi/issues/333).
