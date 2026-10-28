# Roadmap: Redox OS (server) on the Raspberry Pi Zero 2 W

> **Current state (2026-10-28):** v0.2.0 boots on a real Pi 3B+ and a
> Zero 2 W, from the SD card or over the network, with USB keyboard,
> Ethernet and NTP time. The state of each topic is in
> [docs/topics/](docs/topics/README.md), the tasks are
> [GitHub issues](https://github.com/redox-rpi/redox-rpi/issues). The phases this project went through are in
> [docs/roadmap-history.md](docs/roadmap-history.md).

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

## Next release: v0.3.0, stable on the Pi

v0.2.0 added hardware; v0.3.0 makes it dependable. Planned:

1. **Start crashes** ([#1](https://github.com/redox-rpi/redox-rpi/issues/1)–[#3](https://github.com/redox-rpi/redox-rpi/issues/3), [#12](https://github.com/redox-rpi/redox-rpi/issues/12)): up to 12 % of
   program starts fail on the Pi. Done when 1000 starts in a row work.
2. **sdhost as the default SD driver** ([#6](https://github.com/redox-rpi/redox-rpi/issues/6), [#16](https://github.com/redox-rpi/redox-rpi/issues/16)):
   2.5 to 2.9 times faster than `bcm2835-sdhcid`; also fix RedoxFS writing
   36 times the data for small writes.
3. **USB hotplug** ([#17](https://github.com/redox-rpi/redox-rpi/issues/17)): a keyboard or network adapter
   plugged in after boot is found.
4. **Clean boot** ([#18](https://github.com/redox-rpi/redox-rpi/issues/18)): no `pcid` and `audiod` on the Pi.
5. **Full CPU speed** ([#20](https://github.com/redox-rpi/redox-rpi/issues/20)): the ARM clock at 1.4 GHz
   instead of 600 MHz, set through the firmware, with a warning on
   undervoltage or overheating.

If there is time: CI builds with the pinned revisions ([#9](https://github.com/redox-rpi/redox-rpi/issues/9)).
Not planned for v0.3.0: booting the kernel straight from the Pi firmware, without U-Boot ([#15](https://github.com/redox-rpi/redox-rpi/issues/15), v0.4.0), `poweroff` ([#7](https://github.com/redox-rpi/redox-rpi/issues/7), v0.5.0 or later). The Zero 2 W needs no image of its own: the Pi 3B+ image runs on it ([#8](https://github.com/redox-rpi/redox-rpi/issues/8), closed).
Progress: milestone [v0.3.0](https://github.com/redox-rpi/redox-rpi/milestone/1).

## Topics

| Topic | State in one line |
|---|---|
| [Stability](docs/topics/stability.md) | Signal bug during syscalls found and fixed on the Pi (experiment); rare start crash left |
| [relibc](docs/topics/relibc.md) | aarch64 signal entry fixed; tests on Redox missing |
| [SD card](docs/topics/sdcard.md) | sdhost driver works as the only driver (rc1); not yet the default |
| [USB](docs/topics/usb.md) | Hubs, keyboard, bulk transfers; no hotplug yet |
| [Network](docs/topics/network.md) | LAN7800 (Pi 3B+) and RTL8152/RTL8153 (Zero 2 W HAT) |
| [Time](docs/topics/time.md) | NTP at boot with `timesync`; no `clock_settime` yet |
| [Firmware](docs/topics/firmware.md) | `rpi-firmwared` reads board data; CPU clock in progress |
| [Boot](docs/topics/boot.md) | U-Boot, SD card or network; clean boot in progress |
| [Zero 2 W](docs/topics/zero2w.md) | Boots the Pi 3B+ image; slow network boot |
| [CI and tests](docs/topics/ci.md) | QEMU boot in CI; Pi tests through a shared queue |

## Wish list: help wanted

Things that would make the port much more useful. Each is a self-contained
piece of work; if you want to take one on, open an issue first so work is
not done twice. Hardware documentation and datasheets are fine to use, code
from GPL-licensed drivers (Linux) is not.

| Wish | What it takes | Size |
|---|---|---|
| **`poweroff`** | The Pi 3 has no PSCI, so `echo shutdown > /scheme/sys/kstop` only halts one core. Halt through the firmware instead: a watchdog reset that tells the firmware to stay halted (kernel, next to the reset in `bcm2835_wdt.rs`), plus a `poweroff` tool next to `tools/reboot` | small |
| **Prebuilt programs** | bash, nano, vim, tmux, curl, wget … from Redox' aarch64 packages stop with a `brk` exception; relibc's dynamic loader fails to resolve symbols (`libonig.so.5`). Find out why, fix relibc or the packages | medium |
| **Keyboard layouts** | `dwc2d` maps USB keys to a US layout. German and other layouts | small |
| **Setting the time** | `timesync` sets the clock over NTP at boot (v0.2.0). Missing: `clock_settime` / `settimeofday` in relibc, and an I2C clock module (DS3231) for boots without a network ([time.md](docs/topics/time.md)) | small |
| **SSH** | `devshd` is unencrypted and LAN only. A working SSH server for Redox on aarch64 | large |
| **`lsusb` and hardware info** | Show USB devices and board details; model, memory and temperature come from [`/scheme/rpi-firmware`](docs/rpi-firmware.md) | small |
| **Audio** | A sound driver for the Pi: HDMI audio (through the firmware, VCHIQ) and the Pi 3B+ headphone jack (PWM), plus `audiod`, Redox' sound daemon, working on aarch64 on top of it. Until then `audiod` stays out of the image (T18) | medium to large |
| **More firmware services** | `rpi-firmwared` only reads model, memory, clocks, temperature and throttling. Linux also uses the firmware for: power domains (USB, SD on and off, tag `0x28001`; helps `poweroff` and a clean USB reset), the GPIO expander (power LED on the 3B+), the DMA channel mask (`0x60001`, once drivers use DMA) and the display (EDID, resolution, framebuffer; U-Boot sets it up today). See [rpi-firmware.md](docs/rpi-firmware.md) | small each |
| **VCHIQ** | The second VideoCore channel: sound over HDMI and the jack, camera, video decoding. Needed for Audio | large |
| **Mass storage** | USB sticks through `dwc2d` (bulk-only transport, SCSI), [#25](https://github.com/redox-rpi/redox-rpi/issues/25) | medium |
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
