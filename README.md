# redox-rpi

Bringing [Redox OS](https://www.redox-os.org/) (server profile) to the
**Raspberry Pi 3** and **Raspberry Pi Zero 2 W**, with one kernel for both.

The Zero 2 W is close to a Raspberry Pi 3 (BCM2710A1, 4× Cortex-A53, 512 MB
RAM, no onboard Ethernet). Redox already boots on the Pi 3 Model B+, so this
project first gets that port running again on a real Pi 3, then adds a Zero 2 W
board variant and Ethernet networking (DWC2 USB host driver, used by the Pi 3's
onboard Ethernet and by Ethernet/USB hub HATs on the Zero 2 W).

## Status

🚧 Early work. Redox boots to a root shell in QEMU's emulated Raspberry Pi 3
(with a fix from this repository); real hardware is next. See
[docs/status.md](docs/status.md) for details and [ROADMAP.md](ROADMAP.md) for
the step-by-step plan.

Try it on macOS or Linux (needs QEMU, curl and zstd):

```sh
scripts/qemu-raspi3b.sh
```

## Repository layout

| Path | Content |
|---|---|
| [docs/](docs/) | Status and findings, hardware notes, build host setup, [AI challenge draft](docs/ai-challenge.md) |
| [patches/](patches/) | Fixes for upstream components, as `git format-patch` files |
| [scripts/](scripts/) | Helper scripts (QEMU boot) |
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

Issues, ideas and pull requests are welcome. Good starting points are the
open steps in [ROADMAP.md](ROADMAP.md). Please report problems with this fork
here, not to the Redox OS project.

## License

[MIT](LICENSE), like Redox OS itself.
