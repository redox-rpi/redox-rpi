# redox-rpi: Redox OS on the Raspberry Pi 3B+ and Zero 2 W

**Redox OS on real Raspberry Pi 3 Model B+ hardware, with native DWC2 USB
host and LAN7800 Ethernet drivers written in Rust.** Boots on all four
Cortex-A53 cores (aarch64) from the SD card or over the network, with USB
keyboard, Ethernet (DHCP, TCP) and a writable RedoxFS root filesystem.
An unofficial, AI-assisted fork for the Redox OS™ operating system, a Rust
microkernel OS.

Bringing [Redox OS](https://www.redox-os.org/) (server profile) to the
**Raspberry Pi 3** and **Raspberry Pi Zero 2 W**, with one kernel for both.

The Zero 2 W is close to a Raspberry Pi 3 (BCM2710A1, 4× Cortex-A53, 512 MB
RAM, no onboard Ethernet). Redox already boots on the Pi 3 Model B+, so this
project first gets that port running again on a real Pi 3, then adds a Zero 2 W
board variant and Ethernet networking (DWC2 USB host driver, used by the Pi 3's
onboard Ethernet and by Ethernet/USB hub HATs on the Zero 2 W).

## Status

🚧 Alpha. Redox boots from the SD card on a real **Raspberry Pi 3 Model B+**
with all four cores, a USB keyboard, Ethernet and a writable root
filesystem. About every third program still fails at start (a Redox bug on
aarch64, [under investigation](docs/debugging-random-crashes.md)). See
[docs/status.md](docs/status.md), [docs/known-issues.md](docs/known-issues.md)
and [ROADMAP.md](ROADMAP.md).

**Try it on a Pi 3 Model B+:** flash the image from the
[latest release](https://github.com/redox-rpi/redox-rpi/releases) to a microSD
card and log in as `root` / `password`.

**Or in QEMU** on macOS or Linux (needs QEMU, curl and zstd):

```sh
scripts/qemu-raspi3b.sh
```

**For developers:** boot a Pi over the network from your own build server,
no SD card swapping after each build: [docs/netboot.md](docs/netboot.md).

## Repository layout

| Path | Content |
|---|---|
| [docs/](docs/README.md) | Documentation: architecture, status, testing, hardware, build host, [AI challenge draft](docs/ai-challenge.md) |
| [patches/](patches/) | Fixes for upstream components, as `git format-patch` files |
| [scripts/](scripts/) | Build kernel, image and U-Boot; boot in QEMU |
| [config/](config/) | Image and U-Boot configuration |
| [tools/redoxfs-file](tools/redoxfs-file) | Read or replace files in a Redox image without FUSE |

## Goals

- Intermediate goal: Redox boots to a serial shell on the Raspberry Pi 3
- Redox boots to a serial shell on the Zero 2 W
- Persistent root filesystem on the SD card, all 4 cores
- Ethernet on both boards (Wi-Fi later, optional)
- Prebuilt images and a beginner-friendly guide

This is an **unofficial, AI-assisted fork** and not part of the Redox OS
project. Redox does not accept LLM-generated contributions, so changes are not
submitted upstream. They are kept small and close to upstream Redox so the fork
can follow new releases. This repository holds the roadmap, build scripts,
documentation and images.

## Hardware you need

- Raspberry Pi 3 (Model B or B+) and/or Raspberry Pi Zero 2 W (with soldered
  pin header)
- For the Zero 2 W: an Ethernet/USB hub HAT for networking
- 3.3 V USB-to-UART adapter (e.g. CP2102 or FTDI)
- microSD card and a 5 V power supply
- An x86_64 Linux machine (or VM) to build Redox

## Contributing

Help is welcome, also from your AI coding agent: [docs/tasks.md](docs/tasks.md)
lists small, self-contained tasks, and [CONTRIBUTING.md](CONTRIBUTING.md)
describes the workflow (clone, analyse, test, pull request) and how to run
the automated tests in QEMU and on a real Pi. Please report problems with
this fork here, not to the Redox OS project.

## License

[MIT](LICENSE), like Redox OS itself.
