# Roadmap: Redox OS (server) on the Raspberry Pi Zero 2 W

## Starting point

The Pi Zero 2 W is technically almost a **Raspberry Pi 3**: BCM2710A1 SoC
(practically the BCM2837 of the Pi 3), 4× Cortex-A53, but only **512 MB RAM**
and **no onboard Ethernet**.

Redox already has a **Raspberry Pi 3 Model B+ port** (board `raspi3bp`). It
boots via U-Boot to a serial console, with an SD/MMC driver and a PL011 UART.
The hardware list marks it as "Booting" (boots, with issues), and its
documentation is flagged as outdated.

That gives a natural path: **first get the existing Pi 3 port running again,
then add the Zero 2 W as a small board variant.** Both boards share the same
SoC, so almost everything learned on the Pi 3 carries over.

Networking ties the two boards together as well. On both, Ethernet hangs off
the **DWC2 USB controller** through a USB hub:

| Board | Ethernet path |
|---|---|
| Pi 3 Model B | DWC2 → LAN9514 (USB hub + SMSC Ethernet) |
| Pi 3 Model B+ | DWC2 → LAN7515 (USB hub + Microchip Ethernet) |
| Zero 2 W + Ethernet/USB hub HAT | DWC2 → hub chip on the HAT → USB Ethernet chip on the HAT (check with `lsusb` under Linux) |

One DWC2 host driver with hub support unlocks networking on all of them.
Wi-Fi is optional and comes last.

The main hurdles:

1. **DWC2 USB host driver.** Redox's USB stack is built around xHCI; DWC2 is
   new and notoriously complicated. This is the biggest piece of work.
2. **512 MB RAM on the Zero 2 W.** The image has to stay lean.
3. **The Pi 3 port is lightly maintained.** It may need fixing first.

---

## Phase 0: Build environment

| # | Step | Done when … |
|---|---|---|
| 0.1 | Set up an **x86_64 Linux build host** (native machine, VM on x86 hardware or cloud VM; Debian/Ubuntu/Fedora) | Redox build prerequisites installed |
| 0.2 | Native Redox build for x86_64 | `make all` succeeds, `make qemu` boots |
| 0.3 | aarch64 build, run in QEMU (`virt` machine) | Redox boots in `qemu-system-aarch64` |
| 0.4 | Check current state: [issue #1396](https://gitlab.redox-os.org/redox-os/redox/-/issues/1396), HARDWARE.md, open MRs; ask in the Redox Matrix chat who works on ARM | We know what is broken and who maintains it |

Why x86_64 Linux: it is the build host Redox is developed and tested on, and
the prebuilt cross toolchains target it. macOS (including Apple Silicon) only
works through the Podman build, i.e. inside a Linux VM, which is slower and
less tested. The Mac stays useful for editing, flashing SD cards, the serial
console and running QEMU.

## Phase 1: Know the hardware

| # | Step | Done when … |
|---|---|---|
| 1.1 | Accessories: **3.3 V USB-UART adapter** (CP2102/FTDI), microSD cards, power supplies; pin header on the Zero 2 W | – |
| 1.2 | Pi 3: boot Raspberry Pi OS Lite, wire up the serial console (GPIO14/15, `enable_uart=1`, possibly `dtoverlay=disable-bt`) | Linux login over UART |
| 1.3 | Same on the Zero 2 W with the Ethernet/USB hub HAT; note the USB IDs of hub and Ethernet chip (`lsusb`) | Linux login over UART, HAT chips identified |
| 1.4 | Build **U-Boot** for both boards (`rpi_arm64_defconfig`, DTBs `bcm2837-rpi-3-b(-plus).dtb` and `bcm2837-rpi-zero-2-w.dtb`) | U-Boot prompt over UART, `bootefi hello` works |

## Phase 2: Intermediate goal – Redox on the Pi 3

| # | Step | Done when … |
|---|---|---|
| 2.1 | Boot the `raspi3bp` image in **QEMU `-M raspi3b`** | Kernel log on the emulated serial port |
| 2.2 | Same image on the real Pi 3 (if it is a 3B rather than 3B+, swap in the matching DTB) | Kernel log on UART |
| 2.3 | Fix whatever is broken (bootloader, MMU, timer, BCM2836 interrupt controller, UART) | **Shell over the serial console** |
| 2.4 | Root filesystem on the SD card | RedoxFS root on SD, survives reboot |
| 2.5 | Update the outdated Pi 3 docs in the Redox book, submit fixes upstream | Merge requests submitted |

QEMU `raspi3b` is the key tool: development without swapping SD cards, and
automated testing in CI later.

## Phase 3: Redox on the Zero 2 W

| # | Step | Done when … |
|---|---|---|
| 3.1 | Pi 3 image with Zero 2 W DTB and firmware | Kernel log on UART |
| 3.2 | Adjust for 512 MB RAM (memory map, lean image) | Shell over the serial console |
| 3.3 | Own board variant `raspizero2w` in the build system, submitted upstream | Merge request submitted |

## Phase 4: Stable on both boards

| # | Step | Done when … |
|---|---|---|
| 4.1 | Bring up all 4 cores (SMP via spin table) | `top` shows 4 CPUs |
| 4.2 | Mailbox driver (clocks, temperature), watchdog for reboot/shutdown | `reboot` works |
| 4.3 | Minimal "server" profile (low RAM, no GUI) | Runs stable for 24 h |

## Phase 5: Networking over Ethernet

| # | Step | Done when … |
|---|---|---|
| 5.1 | Optional stopgap: IP over the serial line (SLIP/PPP) | Ping from host to Pi |
| 5.2 | **DWC2 USB host driver** (start on the Pi 3, test in QEMU `raspi3b`) | A USB stick or keyboard is enumerated |
| 5.3 | USB hub support behind DWC2 | Devices behind the onboard/HAT hub are enumerated |
| 5.4 | Ethernet driver for the Pi 3 chip (LAN9514 or LAN7515) | `ping` and TCP from the Pi 3 |
| 5.5 | Ethernet driver for the HAT's chip (found in 1.3) | `ping` and TCP from the Zero 2 W |

## Phase 6: Real server and release

- Server services: HTTP server, SSH (depending on available Redox ports), NTP
- Prebuilt images for Pi 3 and Zero 2 W as releases, CI with QEMU `raspi3b`,
  beginner-friendly guide

## Later / optional

- **Wi-Fi** (CYW43436/CYW43438 over SDIO, firmware blob, WPA2): a large
  project of its own
- USB gadget mode (Ethernet over the Zero 2 W's USB cable, no HAT needed)

---

## Open source strategy

- **Upstream first.** Code (kernel, drivers, board config) goes into the
  official **gitlab.redox-os.org** via merge requests, so it does not rot. A
  Redox GitLab account is requested through the Matrix chat.
- **This repository** holds the roadmap, build instructions, scripts, prebuilt
  images and issues.
- **MIT license**, like Redox itself.
- Share progress publicly: Redox Matrix, blog posts, maybe a guest post in the
  Redox news. A working Pi 3 alone already helps many people.

## Risks

- The Pi 3 port is in worse shape than documented: phase 2 takes longer.
- The DWC2 USB controller is notoriously complicated, and QEMU's emulation of
  it is incomplete.
- The HAT's Ethernet chip may need its own driver (if it is not the same as on
  the Pi 3).

Phases 0–4 are realistic in spare time. Phase 5 is a real driver project.

## References

- [Redox Book: AArch64](https://doc.redox-os.org/book/aarch64.html)
- [Redox issue #1396: Raspi3b+ support](https://gitlab.redox-os.org/redox-os/redox/-/issues/1396)
- [RSoC: Porting Redox to AArch64](https://redox-os.org/news/rsoc-arm64-0x01)
