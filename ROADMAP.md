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
| 0.3 | ✅ Own aarch64 image build on the Linux host (`scripts/build-image.sh`, ~25 min) | Image boots in QEMU |
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
| 1.3 | 🟡 Zero 2 W with the Ethernet/USB hub HAT running; chips identified (FE1.1s hub, RTL8152) | Linux login over UART, HAT chips identified |
| 1.4 | 🟡 Build **U-Boot** v2026.10 for both boards (`scripts/build-uboot.sh`, boots Redox in QEMU ✅) | U-Boot prompt over UART on the real boards |
| 1.5 | ✅ Network boot from the build server (`scripts/netboot.sh`, `scripts/make-sdcard.sh`) | Pi 3B+ boots Redox over TFTP |

## Phase 2: Intermediate goal – Redox on the Pi 3

| # | Step | Done when … |
|---|---|---|
| 2.1 | ✅ Boot a Redox aarch64 image in **QEMU `-M raspi3b`** (`scripts/qemu-raspi3b.sh`) | Kernel log on the emulated serial port |
| 2.2 | ✅ Fix the kernel panic in devicetree interrupt parsing (`patches/fdt/`) | Root shell in QEMU `raspi3b` |
| 2.3 | 🟡 Userspace programs run: the signal entry crash is fixed (relibc `0003`, `0004`); some starts still fail, about 2 % in QEMU and up to 12 % on the Pi ([test reports](docs/test-reports.md), [tasks T1–T3](docs/tasks.md)) | No failed start in 1000 |
| 2.4 | 🟡 Same image on the real Pi 3B+: ✅ boots over the network to `redox login:` on HDMI (2026-10-25); login needs the serial console or USB | **Shell over the serial console** |
| 2.5 | 🟡 Root filesystem on the SD card (own `BOARD=raspi3bp` build, starts `bcm2835-sdhcid`); reads and writes clean on the real Pi 3B+ since the data clock fix (2026-10-26) | RedoxFS root on SD without live mode (QEMU ✅), survives reboot |
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
| 4.1 | ✅ Bring up all 4 cores: `spin-table` and BCM2836 IPIs (`patches/kernel/`) | 4 CPUs online (QEMU ✅, real Pi 3B+ ✅) |
| 4.2 | 🟡 Watchdog: ✅ `reboot` (`tools/reboot`); 🟡 firmware driver `rpi-firmwared` (over the VideoCore mailbox) for temperature, clocks and board data (experiment `rpi-firmware`, works in QEMU); `poweroff` still missing | `reboot` and `poweroff` work, `/scheme/rpi-firmware` on the real Pi |
| 4.3 | Minimal "server" profile (low RAM, no GUI) | Runs stable for 24 h |

## Phase 5: Networking over Ethernet

| # | Step | Done when … |
|---|---|---|
| 5.1 | Optional stopgap: IP over the serial line (SLIP/PPP) | Ping from host to Pi |
| 5.2 | 🟡 **DWC2 USB host driver** `dwc2d` (`patches/base/`): host mode, port reset, control transfers over DMA; enumerates the root device in QEMU ✅ and on the real Pi 3B+ ✅ | A USB stick or keyboard is enumerated |
| 5.3 | ✅ USB hub support behind DWC2 (QEMU; real Pi 3B+: both hubs found) | Devices behind the onboard/HAT hub are enumerated |
| 5.3a | ✅ Bulk transfers and a CDC Ethernet adapter in `dwc2d`: DHCP and `ping` in QEMU | Network stack works over USB |
| 5.4 | ✅ Ethernet driver for the Pi 3B+ chip (LAN7800 `0424:7800`) in `dwc2d`: DHCP address, TCP answers from the LAN (2026-10-26) | `ping` and TCP from the Pi 3 |
| 5.5 | 🟡 Ethernet driver for the HAT's chip: RTL8152 `0bda:8152` behind a FE1.1s hub (experiment `rtl8152`: builds, QEMU unaffected; hardware test queued, first on a Pi 3B+ with a USB adapter) | `ping` and TCP from the Zero 2 W |

## Phase 6: Real server and release

- ✅ USB keyboard on the console (split transactions in `dwc2d`)
- ✅ Remote shell over the LAN (`tools/devshd`, port 2323, unencrypted), until SSH works

- Server services: HTTP server, SSH (depending on available Redox ports), NTP
- Prebuilt images for Pi 3 and Zero 2 W as releases, CI with QEMU `raspi3b`,
  beginner-friendly guide

## Wish list: help wanted

Things that would make the port much more useful. Each is a self-contained
piece of work; if you want to take one on, open an issue first so work is
not done twice. Hardware documentation and datasheets are fine to use, code
from GPL-licensed drivers (Linux) is not.

| Wish | What it takes | Size |
|---|---|---|
| **`poweroff`** | The Pi 3 has no PSCI, so `echo shutdown > /scheme/sys/kstop` only halts one core. Halt through the firmware instead: a watchdog reset that tells the firmware to stay halted (kernel, next to the reset in `bcm2835_wdt.rs`), plus a `poweroff` tool next to `tools/reboot` | small |
| **Prebuilt programs** | bash, nano, vim, tmux, curl, wget … from Redox' aarch64 packages stop with a `brk` exception; relibc's dynamic loader fails to resolve symbols (`libonig.so.5`). Find out why, fix relibc or the packages | medium |
| **sdhost driver** | Driver for the Pi's second SD controller (`brcm,bcm2835-sdhost`), the one Linux uses for the card slot. Frees the Arasan controller for Wi-Fi, needed for the Zero 2 W | medium |
| **Keyboard layouts** | `dwc2d` maps USB keys to a US layout. German and other layouts | small |
| **RTC / time** | The Pi has no clock: files are dated 1970/Jan 1. 🟡 NTP client `tools/timesync` with build time and saved time as fallback (experiment `timesync`, works in QEMU); next: `clock_settime` in relibc | small |
| **SSH** | `devshd` is unencrypted and LAN only. A working SSH server for Redox on aarch64 | large |
| **`lsusb` and hardware info** | Show USB devices and board details; model, memory and temperature come from `/scheme/rpi-firmware` (experiment `rpi-firmware`) | small |
| **Zero 2 W Ethernet** | RTL8152 USB Ethernet driver in `dwc2d` (behind the HAT's FE1.1s hub) | medium |
| **Mass storage** | USB sticks through `dwc2d` (bulk-only transport, SCSI) | medium |
| **CI** | Build the image and boot it in QEMU `raspi3b` on every push (`scripts/test-keyboard.sh` is a start) | medium |
| **Wi-Fi** | CYW43438 over SDIO, firmware blob, WPA2 | large |

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
