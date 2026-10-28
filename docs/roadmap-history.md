# Roadmap history: phases 0 to 6

How the project got from an untested Pi 3 port to v0.2.0, as planned in
October 2026. The current plan is in [ROADMAP.md](../ROADMAP.md), the state
of each topic in [topics/](topics/README.md).


## Phase 0: Build environment

| # | Step | Done when … |
|---|---|---|
| 0.1 | Set up an **x86_64 Linux build host** (native machine, VM on x86 hardware or cloud VM; Debian/Ubuntu/Fedora) | Redox build prerequisites installed |
| 0.2 | Native Redox build for x86_64 | `make all` succeeds, `make qemu` boots |
| 0.3 | ✅ Own aarch64 image build on the Linux host (`scripts/build-image.sh`, ~25 min) | Image boots in QEMU |
| 0.4 | ✅ Check current state of the Pi 3 port | [docs/status.md](status.md) |
| 0.5 | ✅ Kernel build and image patching on macOS | [docs/build-host.md](build-host.md), `tools/redoxfs-file` |

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
| 2.3 | 🟡 Userspace programs run: the signal entry crash is fixed (relibc `0003`, `0004`); some starts still fail, about 2 % in QEMU and up to 12 % on the Pi ([test reports](test-reports.md), [tasks T1–T3](tasks.md)) | No failed start in 1000 |
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
| 4.2 | 🟡 Watchdog: ✅ `reboot` (`tools/reboot`); ✅ firmware driver `rpi-firmwared` (over the VideoCore mailbox) for temperature, clocks and board data (base patch `0018`, [tested on the real Pi](rpi-firmware.md)); `poweroff` still missing | `reboot` and `poweroff` work, `/scheme/rpi-firmware` on the real Pi |
| 4.3 | Minimal "server" profile (low RAM, no GUI) | Runs stable for 24 h |

## Phase 5: Networking over Ethernet

| # | Step | Done when … |
|---|---|---|
| 5.1 | Optional stopgap: IP over the serial line (SLIP/PPP) | Ping from host to Pi |
| 5.2 | 🟡 **DWC2 USB host driver** `dwc2d` (`patches/base/`): host mode, port reset, control transfers over DMA; enumerates the root device in QEMU ✅ and on the real Pi 3B+ ✅ | A USB stick or keyboard is enumerated |
| 5.3 | ✅ USB hub support behind DWC2 (QEMU; real Pi 3B+: both hubs found) | Devices behind the onboard/HAT hub are enumerated |
| 5.3a | ✅ Bulk transfers and a CDC Ethernet adapter in `dwc2d`: DHCP and `ping` in QEMU | Network stack works over USB |
| 5.4 | ✅ Ethernet driver for the Pi 3B+ chip (LAN7800 `0424:7800`) in `dwc2d`: DHCP address, TCP answers from the LAN (2026-10-26) | `ping` and TCP from the Pi 3 |
| 5.5 | 🟡 Ethernet driver for the HAT's chip: RTL8152 `0bda:8152` behind a FE1.1s hub (experiment `rtl8152` on branch `rtl8152`, also for the RTL8153: a real RTL8153B passed through to QEMU gets DHCP and answers `ping`; Pi test queued) | `ping` and TCP from the Zero 2 W |

## Phase 6: Real server and release

- ✅ USB keyboard on the console (split transactions in `dwc2d`)
- ✅ Remote shell over the LAN (`tools/devshd`, port 2323, unencrypted), until SSH works

- Server services: HTTP server, SSH (depending on available Redox ports), NTP
- Prebuilt images for Pi 3 and Zero 2 W as releases, CI with QEMU `raspi3b`,
  beginner-friendly guide

