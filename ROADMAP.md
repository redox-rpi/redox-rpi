# Roadmap: Redox OS (server) on the Raspberry Pi Zero 2 W

> **Current state (2026-11-09):** [v0.4.0](https://github.com/redox-rpi/redox-rpi/releases/tag/v0.4.0)
> runs on a real Pi 3B+ and a Zero 2 W, from the SD card or over the
> network, with an optional desktop (beta), sound from the headphone jack
> (beta), fast SD writes and every network adapter at once. We release
> every week ([PMP](docs/process/pmp.md#sprints-and-weekly-releases)).
> The state of each topic is in [docs/topics/](docs/topics/README.md), the
> tasks are [GitHub issues](https://github.com/redox-rpi/redox-rpi/issues).
> The phases this project went through are in
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

## Next release: v0.5.0

Sprint 2026-11-10 to 2026-11-15, release Sunday 2026-11-15
(milestone [v0.5.0](https://github.com/redox-rpi/redox-rpi/milestone/8)).
Already merged: a game (Sopwith) in the desktop instead of the text editor,
with an info window and no yes/no question ([#33](https://github.com/redox-rpi/redox-rpi/issues/33));
TinyCC in the image as a beta ([#293](https://github.com/redox-rpi/redox-rpi/issues/293)).
Planned: our own startup sound ([#292](https://github.com/redox-rpi/redox-rpi/issues/292)),
DNS from DHCP ([#290](https://github.com/redox-rpi/redox-rpi/issues/290)),
MAC address in `ifconfig` ([#171](https://github.com/redox-rpi/redox-rpi/issues/171)),
no Rust backtraces on the console ([#291](https://github.com/redox-rpi/redox-rpi/issues/291)).
What is ready by the cut-off goes in, the rest moves to the next week.

## Themes

Since v0.4.0 we release every week. A release number says what is in it,
not which theme it belongs to: a release with a new feature raises the
minor number, a fixes-only release the patch number, and after v0.9 comes
v0.10 ([release-process.md](docs/release-process.md#numbering)). v1.0
has its own criteria ([RFC 0003](https://github.com/redox-rpi/redox-rpi/pull/289)).
Themes are goals that take several sprints; each has a milestone, and its
parts land in whichever weekly release they are ready for.

| Theme | Contents |
|---|---|
| Done in v0.3.0 and v0.4.0 | stable starts, sdhost, clean boot, USB hotplug, clock follows the load, RedoxFS small writes, sound from the headphone jack (beta), optional desktop (beta), every network adapter at once, multicast |
| Devices | `poweroff` and reboot ([#7](https://github.com/redox-rpi/redox-rpi/issues/7)), USB sticks with `redox-fatfs2` ([#25](https://github.com/redox-rpi/redox-rpi/issues/25)), `pkg install` for Pi packages, keyboard layouts (German first) |
| More boards | Raspberry Pi 400 and Pi 4 (BCM2711), wanted for v1.0 ([pi400.md](docs/topics/pi400.md), [boards.md](docs/topics/boards.md)); Pi 5 later |
| [Network](https://github.com/redox-rpi/redox-rpi/milestone/4) | Wi-Fi ([#26](https://github.com/redox-rpi/redox-rpi/issues/26)), IPv6 ([#27](https://github.com/redox-rpi/redox-rpi/issues/27)), first steps of [RFC 0001](docs/rfcs/0001-identity-and-secure-channels.md): key store `keys:` and encrypted connections `secure:` |
| [Remote access](https://github.com/redox-rpi/redox-rpi/milestone/5) | SSH instead of `devshd`, prebuilt programs (bash, nano; the dynamic loader), `lsusb` and hardware info, registering a phone app with a QR code and logging in with keys (`accounts:`, [RFC 0001](docs/rfcs/0001-identity-and-secure-channels.md)) |
| [Smart home](https://github.com/redox-rpi/redox-rpi/milestone/6) | What homenode and Matter need: mDNS, TLS, GPIO, I2C and SPI, the DS3231 clock, the HomeNode Matter fabric (`rs-matter`, its own project) running on the Pi |
| [Towards 1.0](https://github.com/redox-rpi/redox-rpi/milestone/7) | CI on every push, rebase on current Redox, documentation, the v1.0 criteria of RFC 0003 |

Without a release target: graphics, from a firmware framebuffer to the
GPU ([#33](https://github.com/redox-rpi/redox-rpi/issues/33), [graphics.md](docs/topics/graphics.md)).
Also without a release target: an assistant, a small local language model
that answers questions about Redox and runs a few safe tools
([#43](https://github.com/redox-rpi/redox-rpi/issues/43), [assistant.md](docs/topics/assistant.md)).

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
| [Assistant](docs/topics/assistant.md) | Rust runtime (candle) answers on the Pi (#112, 1 token/s); saved prompt state |

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
| **Audio** | A sound driver for the Pi on top of Redox' `audiod`: first the 3B+ headphone jack through PWM and the DMA controller, then I2S DAC boards, later HDMI audio through `vc4d`. No closed firmware needed. Plan in [audio.md](docs/topics/audio.md) | medium |
| **More firmware services** | `rpi-firmwared` only reads model, memory, clocks, temperature and throttling. Linux also uses the firmware for: power domains (USB, SD on and off, tag `0x28001`; helps `poweroff` and a clean USB reset), the GPIO expander (power LED on the 3B+), the DMA channel mask (`0x60001`, once drivers use DMA) and the display (EDID, resolution, framebuffer; U-Boot sets it up today). See [rpi-firmware.md](docs/rpi-firmware.md) | small each |
| **VCHIQ** | The second VideoCore channel to the closed firmware: camera and video decoding (sound no longer needs it, see [audio.md](docs/topics/audio.md)) | large |
| **Mass storage** | USB sticks through `dwc2d` (bulk-only transport, SCSI), [#25](https://github.com/redox-rpi/redox-rpi/issues/25) | medium |
| **CI** | Build the image and boot it in QEMU `raspi3b` on every push (`scripts/test-keyboard.sh` is a start) | medium |
| **Wi-Fi** | CYW43455 (Pi 3B+) and CYW43436 (Zero 2 W) over SDIO, firmware blob, WPA2; theme Network, [#26](https://github.com/redox-rpi/redox-rpi/issues/26) | large |
| **Bluetooth** | Same chips, over the PL011 UART that is the serial console today; no Bluetooth stack in Redox yet | large |
| **WebAssembly (WASI)** | Run `.wasm` programs on Redox: the same binary runs on Redox, Linux and the Mac, with no compiler on the Pi, each in its own sandbox. First [wasmi](https://github.com/wasmi-labs/wasmi), an interpreter in pure Rust that needs almost nothing from the OS. Later [Wasmtime](https://wasmtime.dev/) with [Cranelift](https://cranelift.dev/) (a code generator in Rust, much smaller than LLVM, with aarch64) for near-native speed; it needs executable memory and trap handling from Redox. An outlook: local apps as WASM modules with an HTML/CSS front end ([Blitz](https://github.com/DioxusLabs/blitz)) and our own host functions for the page, no JavaScript. Web apps from the internet still need a JS engine, because browsers reach the page from WASM only through JavaScript | medium (wasmi), large (Wasmtime, apps) |

## Later / optional

- **Wi-Fi**: theme Network, [#26](https://github.com/redox-rpi/redox-rpi/issues/26)
- USB gadget mode (Ethernet over the Zero 2 W's USB cable, no HAT needed)
- One user on several machines (`sync:`) and personal data shared only after consent (`profile:`, `consent:`), [RFC 0001](docs/rfcs/0001-identity-and-secure-channels.md), [#180](https://github.com/redox-rpi/redox-rpi/issues/180)

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
