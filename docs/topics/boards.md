# More boards and the test lab

Redox should later run on the Pi 4, the Pi 5 and newer boards, not only on
the Pi 3B+ and the Zero 2 W. This page collects what that means for the code
and how a test lab for several boards can run without anyone swapping SD
cards. No work has started; nothing here is planned for a release yet.

## State

- Supported today: Pi 3B+ (BCM2837B0) and Pi Zero 2 W (same SoC family, see
  [zero2w.md](zero2w.md)).
- One Pi 3B+ on homenodellm: network boot from an SD card with firmware and
  U-Boot, serial console on `/dev/ttyUSB0`, power over a Shelly plug, HDMI on
  a NanoKVM ([test-plan.md](../test-plan.md)).
- Firmware tests (open firmware, rows 249-254) need a person to swap the SD
  card. That is the main thing this lab should remove.

## What differs on the newer boards

| Board | SoC | What is new for Redox |
|---|---|---|
| Pi 4 | BCM2711, Cortex-A72 | GIC-400 interrupt controller instead of the Pi 3 one, Ethernet in the SoC (GENET), USB 3 through a PCIe chip (VL805, xHCI), boot loader in an EEPROM on the board |
| Pi 5 | BCM2712, Cortex-A76 | Most I/O (USB, Ethernet, GPIO, UART) sits in the RP1 chip behind PCIe, so PCIe comes first; EEPROM boot loader with network, HTTP and NVMe boot |

Code that only depends on the Cortex-A cores (kernel, relibc) carries over.
Drivers for board peripherals mostly do not: they need new drivers or ports.
Keep board-specific code apart from shared code so a second board does not
mean a fork.

## Test lab without card swaps

1. **Network boot by default.** The Pi 4 and Pi 5 boot over the network
   straight from their EEPROM boot loader, with no SD card at all. Most
   tests would never touch a card.
2. **An SD switch (SD mux) for firmware tests.** The device sits in the Pi's
   card slot and is plugged into homenodellm over USB. The host writes the
   image to the card, then switches the card over to the Pi. Known devices:
   `usbsdmux` (Linux Automation) and SDWire. This also works for the Pi 3B+
   today and would remove the card swaps for the open firmware rows.
3. **Power per board:** one Shelly plug per Pi, or a PoE switch that can turn
   single ports on and off, with a PoE HAT on each Pi.
4. **Serial console per board:** one USB serial adapter each; the Pi 5 has its
   own debug connector for this.
5. **One tool to drive it all:** labgrid knows power switches, serial
   consoles and both SD muxes. The Pi test agent could use it instead of
   per-row shell scripts.

### Compute Modules

CM4 and CM5 with eMMC can be written from the host over USB (`rpiboot`),
with the host setting the boot pin. That makes them good for automated
firmware tests: same SoC as the Pi 4 and Pi 5 (the CM5 also has the RP1).
The board around them is different, though. The CM3+ has no LAN7515
USB/Ethernet chip, which is the centre of our USB and network work on the
3B+, so it does not replace the 3B+. Compute Modules are an extra, not a
replacement for the normal boards.

## Next

- Proposal (needs the user's decision and hardware): one normal board per
  generation (3B+, 4, 5), each with network boot, switchable power and a
  serial console; an SD mux on the 3B+ first, for the firmware rows.

## Later

- Bring-up order on a new board: serial console, timer and interrupts, then
  SD or network, then USB.
- A CM4 or CM5 on an IO board once firmware work moves to the Pi 4 or Pi 5.
