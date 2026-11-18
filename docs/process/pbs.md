# Product breakdown structure (PBS)

What the project delivers, broken down into its parts. The
[WBS](wbs.md) says who builds which part. Status: draft, 2026-11-08; owned
by the project lead and the Architect.

The numbers are stable IDs: requirements, work packages and issues can name
them (for example `PBS 2.3.2`).

## 1 Supported boards

| ID | Board | State |
|---|---|---|
| 1.1 | Raspberry Pi 3B+ (BCM2837B0) | main test board |
| 1.2 | Raspberry Pi Zero 2 W (RP3A0), with a USB Ethernet HAT | runs the Pi 3B+ image |
| 1.3 | Raspberry Pi 3B, 3A+ (BCM2837) | expected to work, not tested |
| 1.4 | Raspberry Pi 4, 400, CM4 (BCM2711) | next board family |
| 1.5 | Raspberry Pi 5, 500, CM5 (BCM2712 + RP1) | after the Pi 4 |

## 2 SD card image

The product a user flashes. Built from the pinned upstream Redox plus our
patches ([config/upstream.lock](../../config/upstream.lock), [patches/](../../patches/README.md)).

### 2.1 Boot chain

| ID | Part | Where |
|---|---|---|
| 2.1.1 | Pi firmware (`bootcode.bin`, `start.elf`) and `config.txt` | [config/config.txt](../../config/config.txt), [rpi-firmware.md](../rpi-firmware.md) |
| 2.1.2 | Open-source VPU firmware (experiment) | [rpi-open-firmware](https://github.com/redox-rpi/rpi-open-firmware) |
| 2.1.3 | U-Boot and the boot menu (SD card or network) | [config/u-boot.config](../../config/u-boot.config), [config/bootmenu.cmd](../../config/bootmenu.cmd) |
| 2.1.4 | Redox bootloader | `patches/bootloader/` |
| 2.1.5 | Device tree | `patches/fdt/` |

### 2.2 Kernel

| ID | Part | Where |
|---|---|---|
| 2.2.1 | Redox microkernel for aarch64 with the BCM2837 patches (interrupts, timer, caches, SMP, watchdog) | `patches/kernel/` |

### 2.3 Drivers (user space, one process each)

| ID | Part | Topic |
|---|---|---|
| 2.3.1 | Serial console (PL011, mini UART) | boot |
| 2.3.2 | USB host (`dwc2d`), hubs, keyboard, mouse, mass storage | usb |
| 2.3.3 | SD card (`sdhost`) | sdcard |
| 2.3.4 | Ethernet: LAN7800 (Pi 3B+), RTL8152/8153 (Zero 2 W HAT) | network |
| 2.3.5 | Wi-Fi (experiment) | network |
| 2.3.6 | Framebuffer (`rpi-fbd`) | graphics |
| 2.3.7 | Display driver for the VideoCore IV HVS ([vc4d](https://github.com/redox-rpi/vc4d)) | graphics |
| 2.3.8 | Firmware interface and CPU clock (`rpi-firmwared`, governor) | firmware |
| 2.3.9 | GPIO, I2C, SPI | gpio |
| 2.3.10 | PWM audio on the headphone jack | audio |
| 2.3.11 | Timers (`timerd`), DMA | time, dma |

### 2.4 System libraries and services

| ID | Part | Topic |
|---|---|---|
| 2.4.1 | relibc, the C library ([redox-rpi/relibc](https://github.com/redox-rpi/relibc)) | relibc |
| 2.4.2 | Network stack (`netstack`), DHCP | network |
| 2.4.3 | RedoxFS | sdcard |
| 2.4.4 | Time: NTP (`timesync`), time zones | time |
| 2.4.5 | Audio server (`audiod`) | audio |
| 2.4.6 | init, getty, login | boot |
| 2.4.7 | Package manager (`pkg`) and the Pi package repository | ci |

### 2.5 User programs

| ID | Part | Topic |
|---|---|---|
| 2.5.1 | Shell (Ion) and core tools | boot |
| 2.5.2 | Pi tools (`reboot`, `cputop`, `ifconfig`, `tone`, …) | per tool |
| 2.5.3 | Native C compiler (TinyCC, experiment) | tcc |
| 2.5.4 | Optional desktop (Orbital with COSMIC apps) | graphics |
| 2.5.5 | Assistant, a local language model (experiment) | assistant |
| 2.5.6 | SSH (planned) | network |

### 2.6 Image configuration

| ID | Part | Where |
|---|---|---|
| 2.6.1 | Board and image configs | [config/raspi3bp.toml](../../config/raspi3bp.toml), `raspi3bp-sdcard.toml`, `raspizero2w.toml` |
| 2.6.2 | Experiments that switch parts on or off | `patches/experiments/` |

## 3 Build and test system

| ID | Part | Where |
|---|---|---|
| 3.1 | Reproducible build (`build-image.sh`, `upstream.lock`, patch sets, manifests) | [versioning.md](../versioning.md) |
| 3.2 | CI: QEMU boot on every push, drift check, nightly | [topics/ci.md](../topics/ci.md) |
| 3.3 | Pi test lab: test Pi, serial console, power plug, HDMI capture, `pi-test.py`, `devshd` (LAN only, never in releases) | [testing.md](../testing.md) |
| 3.4 | Network boot server and image server | [netboot.md](../netboot.md) |
| 3.5 | QEMU for the Pi 3 (`qemu-raspi3b.sh`) | `scripts/` |

## 4 Releases

| ID | Part | Where |
|---|---|---|
| 4.1 | Versioned SD card images (`v0.x.y`, release candidates `-rcN`) with checksums | [GitHub releases](https://github.com/redox-rpi/redox-rpi/releases) |
| 4.2 | Release notes and known issues | [docs/releases/](../releases/), [known-issues.md](../known-issues.md) |

## 5 Documentation

| ID | Part | Where |
|---|---|---|
| 5.1 | User guide and getting started | [user-guide.md](../user-guide.md), [getting-started.md](../getting-started.md) |
| 5.2 | Architecture and topic pages | [architecture.md](../architecture.md), [topics/](../topics/README.md) |
| 5.3 | Process documents (PMP, QMP, standards, requirements, PBS, WBS) | [docs/process/](.) |
| 5.4 | Org profile and join page | [redox-rpi/.github](https://github.com/redox-rpi/.github), [join.md](../join.md) |
