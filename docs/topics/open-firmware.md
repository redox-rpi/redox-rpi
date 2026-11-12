# Open firmware

Replacing everything closed from Broadcom on the Pi, as far as possible
([#230](https://github.com/redox-rpi/redox-rpi/issues/230)). Only the Boot
ROM in the chip stays.

The firmware is its own sub-project of redox-rpi:
[redox-rpi/rpi-open-firmware](https://github.com/redox-rpi/rpi-open-firmware)
(GPLv2+). It has its own docs, its own Pi tests and its own pace. Almost
nothing in it is Redox-specific: it starts U-Boot, Linux or Redox all the
same ([#333](https://github.com/redox-rpi/redox-rpi/issues/333)). This page
says **what Redox uses from the firmware and how**. How the firmware works
inside is in that repository:

- [Pi models](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/models.md)
  and [Broadcom's firmware per model](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/broadcom-firmware.md)
- [Boot chain by release](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/boot-chain.md)
  (with pictures)
- [Our firmware](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/own-firmware.md)
  and [Pi test results](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/test-results.md)

## What Redox uses from the firmware today

Every redox-rpi image boots with Broadcom's firmware, version
`1.20260915`. `scripts/rpi-firmware.sh` fetches it from Raspberry Pi's
release and puts it on the card. For the Pi 3B+ and Zero 2 W:

| Piece | Who runs it, when | What Redox needs from it | Open replacement |
|---|---|---|---|
| Boot ROM | VPU, power-on | loads `bootcode.bin` from the card | none (inside the chip) |
| `bootcode.bin` | VPU | loads `start_cd.elf` | ours: M1 passed; on the test Pi as the boot selector only |
| `start_cd.elf` + `fixup_cd.dat` | VPU (ThreadX) | SDRAM and clocks; reads `config.txt`; loads `u-boot.bin` (v0.4.0) or `kernel8.img` with `initramfs` (v0.5.0 direct boot) and the devicetree; writes memory size, serial number and MAC address into the devicetree | ours: `boot64` loads stub, kernel and devicetree from FAT and fills in the memory size (rpi-open-firmware PR #9, host-tested, Pi row 332 queued) |
| `armstub8` (built into `start_cd.elf`) | ARM, EL3, before the kernel | timer frequency, EL3 to EL2, parks cores 1–3 in the spin table the Redox kernel releases | ours: `armstub8-redox.bin`, booted Redox with Broadcom's firmware (PR #4) |
| Devicetree `bcm2710-rpi-3-b-plus.dtb`, overlays `disable-bt`, `mmc` | read by U-Boot and the Redox kernel and drivers | where the hardware is | already open source (Linux); only the values `start.elf` writes in are missing without it |
| **Mailbox** (`start_cd.elf` keeps running while Redox runs) | VPU, at any time | `rpi-firmwared`: temperature, throttling, clocks, raising the ARM clock from 600 to 1400 MHz, board data ([rpi-firmware.md](../rpi-firmware.md)); U-Boot: memory size, framebuffer; graphics: framebuffer and EDID ([graphics](graphics.md)); `rpi-update try`: tryboot flag | M3, not started. The biggest gap: without it there is no fast ARM clock and no screen. |
| HDMI set-up | VPU, at power-on | the screen mode for the framebuffer | M4, not started |
| Wi-Fi and Bluetooth blobs | the wireless chip | not used yet (Bluetooth: [#54](https://github.com/redox-rpi/redox-rpi/issues/54)) | not possible: they run inside the wireless chip |

## How the open firmware will reach Redox

1. **Test Pi only (now):** the card's `bootcode.bin` is our boot
   selector. Enter on the serial console opens our monitor, where
   firmware tests run with no card swap. Otherwise Broadcom's
   `bootcode.bin` (built in) boots Redox as before.
2. **Our `armstub8`** in a release card: one `armstub=` line in
   `config.txt`, Broadcom's firmware otherwise. This is the first open
   piece a release can take (candidate for v0.6.0).
3. **Our VPU firmware boots Redox** (M2 plus M3): `bootcode.bin` loads
   `armstub8`, `kernel8.img` and the devicetree itself. That needs the
   mailbox (M3) first, or `rpi-firmwared`, the ARM clock and the screen
   stop working.
4. **End goal:** no Broadcom code after the Boot ROM, on every model that
   allows it ([per model](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/broadcom-firmware.md#every-closed-piece-and-whether-it-can-be-replaced)).

The interface proposed in #333: redox-rpi takes only tagged releases
of rpi-open-firmware, never a branch, the same way it takes Broadcom's
firmware today. Broadcom's firmware stays on the card as the fallback.

## State

- Test Pi: boot selector passed (2026-11-11).
- M2: ARM start in AArch64 (PR #7), SD card and FAT (PR #8), `boot64`
  (PR #9), all in rpi-open-firmware. They wait for Pi row 332.
- Next: M3 mailbox service, then M4 HDMI.
- M4 reference values: `socdump regs` under Broadcom's firmware and
  `socdump diff` against ours ([tools/socdump](../../tools/socdump/README.md),
  [#224](https://github.com/redox-rpi/redox-rpi/issues/224)).
