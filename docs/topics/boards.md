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

The Pi 400 (BCM2711, same as the Pi 4) has its own plan for a first test
version: [pi400.md](pi400.md), work package [#299](https://github.com/redox-rpi/redox-rpi/issues/299).

## Why the Pi Zero W (and Pi 1) does not run Redox

The Pi Zero W looks like a small Zero 2 W but has an older chip. Draft
assessment, 2026-11-09; no work is planned.

| | Pi Zero W | Pi Zero 2 W (supported) |
|---|---|---|
| SoC | BCM2835 (as in the Pi 1) | BCM2710A1 (Pi 3 family) |
| CPU | 1× ARM1176JZF-S, 1 GHz, **ARMv6, 32-bit only** | 4× Cortex-A53, AArch64 |
| RAM | 512 MB | 512 MB |
| Interrupts, timer | BCM2835 interrupt controller, system timer, no ARM generic timer | Pi 3 local interrupt controller, ARM generic timer |
| Peripherals at | 0x20000000 | 0x3f000000 |

Redox runs on x86_64, i586, AArch64 and RISC-V 64. It has no 32-bit ARM
port, and the ARM1176 cannot run AArch64 code. Our whole port builds on
AArch64. Booting the Zero W would need a new architecture, almost all of
it in upstream code rather than in our Pi layer:

1. **Kernel:** an `arm` (ARMv6) architecture: short-descriptor page tables
   (VMSAv6), exception vectors, context switch, syscall ABI, the BCM2835
   interrupt controller and system timer, no SMP.
2. **Toolchain and relibc:** an `arm-unknown-redox` Rust target (ARMv6,
   VFPv2 hard float) with build-std; relibc for 32-bit ARM (syscalls,
   startup, setjmp, TLS). Code that assumes 64-bit pointers will break in
   relibc and in many packages.
3. **Boot:** U-Boot has `rpi_0_w_defconfig` with EFI, but the Redox
   bootloader would need a 32-bit ARM UEFI build, or the firmware would load
   the kernel directly.
4. **Userspace:** every recipe built and fixed for the new target.
5. **Drivers:** the small part. `bcm2835-sdhostd`, `dwc2d`,
   `bcm2835-pwmaudiod` and GPIO already know the BCM2835 blocks; only the
   addresses change, and they come from the device tree. WLAN (CYW43438
   over SDIO) needs a driver the Pi 3 and Zero 2 W also lack.

That is months of work for a single-core board with 512 MB, with nearly
all the effort outside our Pi-specific code. For a small board, use the
Zero 2 W.

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
