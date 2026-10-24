# Roadmap: Redox OS (server) on the Raspberry Pi Zero 2 W

> **Current state (2026-10-24):** the daily Redox server image boots in QEMU
> `raspi3b` to a root shell, with a fix for a kernel panic on the Pi's
> devicetree. Details in [docs/status.md](docs/status.md). Done steps are
> marked ✅, partly done ones 🟡.

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
| 0.3 | 🟡 aarch64 build, run in QEMU (`virt` machine) | Redox boots in `qemu-system-aarch64` (daily image ✅, own build open) |
| 0.4 | ✅ Check current state of the Pi 3 port | [docs/status.md](docs/status.md) |
| 0.5 | ✅ Kernel build and image patching on macOS | [docs/build-host.md](docs/build-host.md), `tools/redoxfs-file` |

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
| 2.1 | ✅ Boot a Redox aarch64 image in **QEMU `-M raspi3b`** (`scripts/qemu-raspi3b.sh`) | Kernel log on the emulated serial port |
| 2.2 | ✅ Fix the kernel panic in devicetree interrupt parsing (`patches/fdt/`) | Root shell in QEMU `raspi3b` |
| 2.3 | Userspace programs run (relibc `no filetable fd present` crash, upstream regression) | `ls`, `ps`, `free` work in QEMU |
| 2.4 | Same image on the real Pi 3 (if it is a 3B rather than 3B+, swap in the matching DTB) | **Shell over the serial console** |
| 2.5 | Root filesystem on the SD card (own `BOARD=raspi3bp` build, starts `bcm2835-sdhcid`) | RedoxFS root on SD, survives reboot |
| 2.6 | Document the Pi 3 state and all fixes in this repository | Step-by-step guide in `docs/` |

QEMU `raspi3b` is the key tool: development without swapping SD cards, and
automated testing in CI later.

## Phase 3: Redox on the Zero 2 W

| # | Step | Done when … |
|---|---|---|
| 3.1 | U-Boot that uses the firmware's DT (the prebuilt one embeds a Pi 3B DT) | U-Boot prompt, correct model and RAM size |
| 3.2 | Pi 3 image with Zero 2 W DTB and firmware | Kernel log on UART |
| 3.3 | Adjust for 512 MB RAM: the 509 MiB server live image does not fit, use a smaller image or boot from SD | Shell over the serial console |
| 3.4 | SD card: route it to the Arasan controller (`dtoverlay=mmc`?) or write an sdhost driver | Root filesystem on SD |
| 3.5 | Own board variant `raspizero2w` in the fork's build system | `make` builds a Zero 2 W image |

## Phase 4: Stable on both boards

| # | Step | Done when … |
|---|---|---|
| 4.1 | 🟡 Bring up all 4 cores: `spin-table` and BCM2836 IPIs (`patches/kernel/`) | 4 CPUs online (QEMU ✅, real Pi 3 open) |
| 4.2 | Mailbox driver (clocks, temperature), watchdog for reboot/shutdown | `reboot` works |
| 4.3 | Minimal "server" profile (low RAM, no GUI) | Runs stable for 24 h |

## Phase 5: Networking over Ethernet

| # | Step | Done when … |
|---|---|---|
| 5.1 | Optional stopgap: IP over the serial line (SLIP/PPP) | Ping from host to Pi |
| 5.2 | **DWC2 USB host driver** (start on the Pi 3, test in QEMU `raspi3b`) | A USB stick or keyboard is enumerated |
| 5.3 | USB hub support behind DWC2 | Devices behind the onboard/HAT hub are enumerated |
| 5.4 | Ethernet driver for the Pi 3 chip (LAN7800 `0424:7800` on our 3B+, LAN9514 on the 3B) | `ping` and TCP from the Pi 3 |
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

- **An unofficial, AI-assisted fork.** Development uses AI coding tools. Since
  March 2026 Redox does not accept LLM-generated contributions, so this work
  lives in a fork and is not submitted upstream. The fork is clearly labelled
  as unofficial and AI-assisted, and does not claim to be Redox OS itself.
- **Stay close to upstream.** Changes are kept small and in separate commits
  per component (kernel, drivers, board config), so the fork can be rebased on
  new Redox releases and parts could be rewritten for upstream later if the
  policy changes or someone reimplements them by hand.
- **This repository** holds the roadmap, build instructions, scripts, prebuilt
  images and issues; forked Redox components are linked from here.
- **MIT license**, like Redox itself. No code copied from GPL-licensed sources
  (e.g. Linux drivers); hardware documentation and datasheets are fine.
- Share progress publicly: blog posts, forums, Raspberry Pi community. A
  working Pi 3 alone already helps many people.

## Risks

- The Pi 3 port is in worse shape than documented: phase 2 takes longer.
- The DWC2 USB controller is notoriously complicated, and QEMU's emulation of
  it is incomplete.
- The HAT's Ethernet chip may need its own driver (if it is not the same as on
  the Pi 3).
- The fork drifts away from upstream Redox if it is not rebased regularly.

Phases 0–4 are realistic in spare time. Phase 5 is a real driver project.

## References

- [Redox Book: AArch64](https://doc.redox-os.org/book/aarch64.html)
- [Redox issue #1396: Raspi3b+ support](https://gitlab.redox-os.org/redox-os/redox/-/issues/1396)
- [RSoC: Porting Redox to AArch64](https://redox-os.org/news/rsoc-arm64-0x01)
