# Wi-Fi

An effort analysis, written 2026-10-29 before any work started
([#26](https://github.com/redox-rpi/redox-rpi/issues/26), milestone v0.6.0).
Mainly the Pi 3B+ (CYW43455); the Zero 2 W (CYW43436) uses the same
driver with other firmware files. Sizes are rough guesses for this
project's way of working (agent sessions, one Pi on the test network).

## State

Nothing exists:

- Redox has no Wi-Fi driver for any chip, no SDIO code, and nothing in
  `netstack` about wireless. `wpa_supplicant` is an unbuilt recipe
  (`recipes/wip/services/wpa_supplicant`).
- `netstack` opens only the first `network.*` scheme it finds
  (`get_network_adapter`, "FIXME allow using multiple network adapters")
  and names the interface `eth0` in about eight places. So until it
  handles several adapters, Wi-Fi and Ethernet are either-or.
- The Arasan controller, which the Wi-Fi chip hangs off on the Pi 3B+, is
  in use: `bcm2835-sdhcid` reads the SD card with it. It becomes free once
  sdhost is the default SD driver
  ([#6](https://github.com/redox-rpi/redox-rpi/issues/6)).

## Hardware and wiring (Pi 3B+)

From the Linux device tree (`bcm2837-rpi-3-b-plus.dts`,
`bcm283x-rpi-wifi-bt.dtsi`):

| Part | Wiring |
|---|---|
| SD card | sdhost controller, GPIO 48–53 (after #6) |
| Wi-Fi (CYW43455) | Arasan SDHCI (`mmc@7e300000`), GPIO 34–39 in ALT3 (`emmc_gpio34`), 4-bit bus, non-removable, function 1 is `brcm,bcm4329-fmac` |
| Wi-Fi power | `wifi_pwrseq`: reset line on the firmware's expander GPIO 1 (`expgpio`), set through the mailbox, so `rpi-firmwared` can do it |
| Bluetooth | PL011 UART (`uart0`), today the serial console; not part of this |

The SDIO host is therefore a second user of the Arasan registers that
`bcm2835-sdhcid` drives today. The plan is a new driver (`bcm2835-sdiod`
or similar) that only starts when the SD card is on sdhost. It would copy
the command and clock code from `bcm2835-sdhcid` (about 900 lines in
`sd/mod.rs`) rather than share it, so the SD driver files stay untouched.
Whether sharing is better is a question for the SD session before step 1
starts.

Zero 2 W: the same wiring (CYW43436 on the Arasan controller, SD card
on sdhost in Linux), so #6 frees the controller there too.

## Firmware and licence

| File | Size | Licence |
|---|---|---|
| `cyfmac43455-sdio-standard.bin` (firmware) | 609309 bytes | Cypress binary licence |
| `cyfmac43455-sdio-minimal.bin` (smaller variant) | 548728 bytes | Cypress binary licence |
| `cyfmac43455-sdio.clm_blob` (regulatory data) | 2676 bytes | Cypress binary licence |
| `brcmfmac43455-sdio.raspberrypi,3-model-b-plus.txt` (NVRAM, board settings) | a few KB | GPL-2 |

Source: [RPi-Distro/firmware-nonfree](https://github.com/RPi-Distro/firmware-nonfree)
(`debian/config/brcm80211/cypress/`, `brcm/`). Raspberry Pi OS ships them.

The Cypress licence (`LICENCE.cypress` there) allows redistribution in
object code form, only for use with Cypress chips, and forbids modifying
or reverse engineering the files. That is fine for a recipe that
downloads the files and puts them on the image, like the Pi boot firmware
today. They must not go into this repository's source tree, and the
release notes must name the licence.

The NVRAM file is GPL-2 text data, not code: shipping it unchanged next to
our MIT code is fine, the same as the GPL `start.elf` neighbours on the
boot partition. We must not copy it into a Rust source file.

## Driver steps

The chip is FullMAC: scanning, association, rate control and (maybe) WPA2
run in the chip's firmware. The host loads the firmware, sends commands
("iovars") and gets plain 802.3 Ethernet frames back. That keeps the
driver small compared to a SoftMAC chip.

1. **SDIO host** on the Arasan controller: card init (CMD5, CMD3, CMD7),
   CMD52 (single byte) and CMD53 (block transfers), 4-bit bus, 25 to 50 MHz,
   the card interrupt. PIO first; DMA after
   [#21](https://github.com/redox-rpi/redox-rpi/issues/21).
   **800–1200 lines.**
2. **Chip bring-up and firmware load:** power on through `rpi-firmwared`,
   the backplane window (the chip's AXI bus behind function 1), read the
   chip ID (0x4345), halt the ARM core, write firmware and NVRAM into chip
   RAM, release the core, wait for the "HT clock" and the firmware's ready
   signal. **600–900 lines.**
3. **Protocol:** SDPCM framing with credits on function 2, BCDC control
   messages, the iovar get/set helpers, firmware events (link up/down,
   scan results). Load the CLM blob. **600–800 lines.**
4. **Scan and join:** `escan`, join an open network, then a WPA2 network
   through the firmware supplicant (see below). **400–600 lines.**
5. **Network adapter:** offer `/scheme/network.wlan0` like the Ethernet
   drivers (`driver-network`), with the MAC from the chip. Data frames
   need only the SDPCM/BCDC header stripped. **150–300 lines.**
6. **Settings:** network name and password in a config file, a small tool
   to scan and pick a network. **200–300 lines.**

### WPA2

Two ways:

- **Firmware supplicant (preferred).** Linux `brcmfmac` uses it when the
  firmware answers the `sup_wpa` iovar (its `FWSUP` feature): the host
  sets the passphrase (`wsec_pmk`) and the chip does the 4-way handshake
  itself. Then WPA2 costs almost nothing extra. Whether the Pi's
  `cyfmac43455-sdio-standard.bin` has it is **unknown**; it is the first
  thing to ask the chip once the "ver" iovar answers (step 2). WPA3-SAE
  would need the `sae` feature on top.
- **Host supplicant (fallback).** If the firmware has no supplicant, the
  driver must pass EAPOL frames up and something must do PBKDF2 for the
  PMK, the 4-way and group handshakes, and install the keys with iovars.
  Porting `wpa_supplicant` (BSD licence) to Redox means its driver
  interface, nl80211-free, plus relibc gaps; a small Rust handshake of our
  own is likely less work: **800–1200 lines** (PBKDF2-SHA1, HMAC, AES key
  wrap from existing crates).

### Netstack

- The adapter looks like Ethernet, so `netstack` works unchanged if
  `network.wlan0` is the only adapter (no USB Ethernet plugged in, or the
  Ethernet driver not started).
- Wi-Fi and Ethernet at the same time needs `netstack` to open every
  adapter, give each its own interface name and routes, and `dhcpd` to run
  per interface. That is a change of its own, also useful for USB Ethernet
  plus the onboard LAN7800, and best done upstream: **300–600 lines**.
- Link up/down must reach `dhcpd`, so a new network gets a new lease.

## Other systems on the Pi

| System | Wi-Fi on the Pi 3 | Notes |
|---|---|---|
| Linux | `brcmfmac` (ISC) | reference for everything; FWSUP and SAE through firmware features |
| OpenBSD | `bwfm(4)` (ISC) | supports BCM43455 over SDIO; `if_bwfm_sdio.c` 1536 lines, `bwfm.c` 3324 lines; WPA2 in the host (net80211) |
| NetBSD | `bwfm(4)`, from OpenBSD | same code base |
| Plan 9 (Pi port) | `ether4330` | one small driver, WPA2 in user space |
| Circle (C++ bare metal) | `addon/wlan`, port of Plan 9's `ether4330` plus `wpa_supplicant` | shows a small bare-metal driver is enough |

OpenBSD `bwfm` is the best template: ISC licensed, small, and the same
split (SDIO glue plus a common FullMAC core) this page proposes. Linux
`brcmfmac` is ISC too, but larger; use it to check firmware details.
No code from GPL sources (CONTRIBUTING.md).

## Effort estimate

| Piece | Lines | Estimate |
|---|---|---|
| SDIO host | 800–1200 | 1–2 weeks |
| Chip bring-up and firmware load | 600–900 | 1–2 weeks |
| SDPCM/BCDC protocol, iovars, events | 600–800 | about 1 week |
| Scan and join, firmware WPA2 | 400–600 | days |
| Network adapter | 150–300 | days |
| Settings tool | 200–300 | days |
| **Sum with firmware supplicant** | **about 3000–4000** | **1–2 months** |
| Host supplicant, if needed | 800–1200 | +2–3 weeks |
| `netstack` with several adapters | 300–600 | +1–2 weeks |

These estimates are calendar time in the style of the other pages. The
graphics page showed that agent sessions write code with a Redox template
much faster than guessed; here there is no template in Redox for SDIO or
Wi-Fi, and QEMU emulates neither the Arasan SDIO mode nor the chip. So
nearly all testing is on the real Pi, and each round is a build plus a
netboot (about an hour with the queue free). The tests need no person:
the serial console and the Shelly plug are enough, and the Fritz!Box is
the access point.

The Wi-Fi password for the test network is a secret: it goes into the
image on homenodellm at build time, never into the repository or a test
log.

## Next

Not started. Milestones, each a Pi test:

1. After #6: the SDIO driver enumerates the chip and reads chip ID 0x4345.
2. Firmware and NVRAM loaded, the "ver" iovar answers; check `sup_wpa`.
3. A scan lists the networks around the test Pi.
4. Join the Fritz!Box over WPA2 with the firmware supplicant, `dhcpd`
   gets an address, `scripts/pi-test.py` runs over Wi-Fi (Ethernet cable
   out).
5. Settings tool and config file; then the Zero 2 W with its own firmware
   files, once a Zero is on the network.

## Later

- Wi-Fi and Ethernet at the same time (`netstack` with several adapters).
- WPA3-SAE, if the firmware offers it.
- Access point mode.
- Bluetooth (needs the PL011 UART, today the serial console).

## Sources

- Linux device tree: `arch/arm/boot/dts/broadcom/bcm2837-rpi-3-b-plus.dts`,
  `bcm283x-rpi-wifi-bt.dtsi` (wiring only, no code taken)
- [RPi-Distro/firmware-nonfree](https://github.com/RPi-Distro/firmware-nonfree)
  (firmware files, `LICENCE.cypress`)
- [OpenBSD bwfm(4)](https://man.openbsd.org/bwfm.4), `sys/dev/sdmmc/if_bwfm_sdio.c`,
  `sys/dev/ic/bwfm.c` (ISC)
- Linux `drivers/net/wireless/broadcom/brcm80211/brcmfmac/` (ISC), feature
  detection in `feature.c` (`sup_wpa`, `sae`)
- SD Association, SDIO Simplified Specification (CMD5, CMD52, CMD53)
- Upstream Redox `base`: `netstack/src/main.rs` (one adapter only),
  `drivers/storage/bcm2835-sdhcid` (sizes counted 2026-10-29)
