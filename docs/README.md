# redox-rpi documentation

Everything about running Redox OS on the Raspberry Pi 3 and Zero 2 W: how
the pieces fit together, how to build and test, and what we found on the way.

## Start here

| Page | What it explains |
|---|---|
| [Getting started](getting-started.md) | From the download to the first login: what you need, writing the SD card, serial console or screen, first boot, first commands, reporting problems |
| [User guide](user-guide.md) | Using the image: logging in, the Ion shell, schemes, network, time, `pkg`, reboot, logs |
| [Time](time.md) | The clock at boot, `timesync`, setting the time by hand, time zones, timers (`timerd`), the DS3231 clock module |
| [Smart home](smarthome.md) | MQTT with `mqtt-pub` and `mqtt-sub`, the Pi in Home Assistant with `ha-bridge` |
| [Architecture](architecture.md) | The boot chain from power-on to the shell, every component and where it comes from |
| [Status and findings](status.md) | What works, what does not, bugs found and fixed |
| [Roadmap](../ROADMAP.md) | The step-by-step plan |

## How to

| Page | What it explains |
|---|---|
| [Testing](testing.md) | Booting Redox in QEMU (macOS and Linux), automated boot test, CI |
| [Network boot](netboot.md) | For developers: booting a real Pi from your own build server over the network, the SD card for it |
| [Build host](build-host.md) | Setting up the Linux build machine, building the kernel on macOS |
| [Hardware](hardware.md) | Boards, serial console wiring, `config.txt`, Ethernet/USB topology |
| [Firmware properties](rpi-firmware.md) | `/scheme/rpi-firmware`: temperature, clocks, throttling and board data from the Pi firmware |
| [Versioning](versioning.md) | Pinned upstream revisions, experiments, build IDs and manifests |
| [Test plan](test-plan.md) | Who has the test Pi now and which test comes next |
| [Test reports](test-reports.md) | Results of the standard tests on a real Pi, per build |
| [Open tasks](tasks.md) | Small tasks for contributors and their AI agents; workflow in [CONTRIBUTING.md](../CONTRIBUTING.md) |

## Background

| Page | What it explains |
|---|---|
| [AI challenge (draft)](ai-challenge.md) | Idea for a benchmark: AI agents porting an OS to the Pi |

## Glossary

| Term | Meaning |
|---|---|
| **BCM2837 / BCM2710A1** | The Broadcom chip of the Pi 3 / Zero 2 W: 4× ARM Cortex-A53 |
| **Devicetree (DT, `.dtb`)** | A description of the hardware that the firmware passes to the OS, so the kernel knows which devices exist and where |
| **U-Boot** | Bootloader for embedded boards; on the Pi it provides UEFI so the Redox bootloader can run |
| **UEFI** | Standard firmware interface, also used by PCs; the Redox bootloader is a UEFI program |
| **RedoxFS** | Redox's own filesystem |
| **initfs** | Small filesystem loaded with the kernel, holds `init` and the drivers needed to reach the real root filesystem |
| **Live mode** | The bootloader copies the whole filesystem into RAM; no SD card driver needed, but slow and RAM-hungry |
| **PL011 / mini UART** | The Pi's two serial ports; Redox only supports the PL011 |
| **SMP** | Using several CPU cores |
| **spin-table** | How the Pi firmware parks the extra cores until the OS releases them |
| **IPI** | Inter-processor interrupt, how cores notify each other |
| **DWC2** | The Pi's USB controller; Ethernet on the Pi 3 and the Zero 2 W HAT hangs off it |
| **relibc** | Redox's C library, used by almost every program |
