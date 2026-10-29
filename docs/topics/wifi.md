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

Which firmware has a supplicant, checked offline on 2026-10-29 from the
build tag at the end of each file (the feature list the firmware was
built with) in RPi-Distro/firmware-nonfree:

| File | Version | Build tag features | Firmware supplicant |
|---|---|---|---|
| `cyfmac43455-sdio-standard.bin` | 7.45.265 (2023-08-29) | `…-mfp-…-gtkoe-roamprof-txbf-ve-extsae-dpp-sr-okc-bpd` | **no** (`idsup` missing); `extsae` means SAE done by the host |
| `cyfmac43455-sdio-minimal.bin` | 7.45.241 (2021-11-01) | `…-wl11u-wfds-mfp-wowlpf-idsup-idauth-maxclients-…` | **yes**: `idsup` (in-dongle supplicant, what `sup_wpa` reports) and `idauth` |

- Raspberry Pi OS installs both and picks **standard** by default
  (`update-alternatives`, priority 50 against 10 for minimal).
- It also loads `brcmfmac` with `feature_disable=0x282000`. In the Linux
  feature list that is bit 13 `FWSUP`, bit 19 `SAE` and bit 21
  `DUMP_OBSS`: Pi OS switches the firmware supplicant off even where it
  exists and always uses host `wpa_supplicant`. Why is not documented
  (the same file also turns roaming off "for WLAN stability").
- The iovar name `sup_wpa` itself is not in either file; the iovar tables
  are in the chip's ROM ("roml" in the tag). So the build tag is the
  evidence, and the real answer comes from asking the chip in step 2.

Two ways, then:

- **Firmware supplicant with the minimal file (cheapest).** The host sets
  the passphrase (`wsec_pmk`) and the chip does the 4-way handshake
  itself, so WPA2-PSK costs almost nothing extra. The price: the older
  2021 firmware, no WPA3 (no SAE in that file), and a path Pi OS avoids,
  so stability problems would be ours to find.
- **Host supplicant with the standard file (what Pi OS does).** The
  driver passes EAPOL frames up, and something does PBKDF2 for the PMK,
  the 4-way and group handshakes, and installs the keys with iovars.
  Porting `wpa_supplicant` (BSD licence) to Redox means its driver
  interface, nl80211-free, plus relibc gaps; a small Rust handshake of our
  own is likely less work: **800–1200 lines** (PBKDF2-SHA1, HMAC, AES key
  wrap from existing crates). WPA3-SAE later through `extsae` (the host
  does SAE, the firmware the rest).

Plan: start with the minimal file and the firmware supplicant to get a
first WPA2 join quickly, then decide on the host supplicant once it runs.

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
| **Sum with firmware supplicant (minimal file)** | **about 3000–4000** | **1–2 months** |
| Host supplicant (standard file, WPA3 later) | 800–1200 | +2–3 weeks |
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

## In progress

Step 1 on branch `wifi` (on top of `sdhost-ctxleak`), experiment
[`wifi-sdio`](../../patches/experiments/wifi-sdio/README.md):

- `rpi-firmwared` gets `/scheme/rpi-firmware/wifi_power`, WL_REG_ON on
  firmware expander GPIO 129 (base `0030`).
- `bcm2835-sdiod` (base `0031`), a probe that runs once at boot: GPIO 34-39
  to the Arasan, SDIO init at 400 kHz, CCCR and CIS, function 1, ALP
  clock, and the ChipCommon chip ID over CMD52 and CMD53, then again at
  25 MHz on 4 data lines. It refuses to run when the SD card is on the
  Arasan (GPIO 48-53 ALT3).
- QEMU (no Wi-Fi chip): starts and stops with a CMD5 timeout, the card on
  sdhost keeps working. Pi test #57 is queued for the Pi test agent
  (image `v0.3.0-dev-216-gfb78c4a+xd4e7`).

Step 2 (base `0032`, Pi test #60 after #57) and steps 3–4 (base `0033`,
scan and WPA2 join) are written. They build and their unit tests pass;
nothing of them has run against the chip yet.

### Step 2: firmware start, version and MAC address

**Firmware files.** Recipe `wifi-firmware` (in the experiment's `recipes/`
directory; `build-image.sh` copies experiment recipes into the cookbook)
downloads at build time, from pinned commits, and checks SHA-256:

| File on the image (`/usr/lib/firmware/brcm/`) | From |
|---|---|
| `cyfmac43455-sdio-minimal.bin`, `-standard.bin`, `cyfmac43455-sdio.clm_blob` | RPi-Distro/firmware-nonfree `c91cd28`, `debian/config/brcm80211/cypress/` |
| `brcmfmac43455-sdio.raspberrypi,3-model-b-plus.txt` | same, `brcm/brcmfmac43455-sdio.txt` (the 3B+ name is a link to it) |
| `LICENCE.cypress` | linux-firmware tag `20250311` (firmware-nonfree has no copy) |

The driver loads the minimal firmware (in-chip supplicant, see WPA2);
`bcm2835-sdiod standard` takes the other one.

**Sequence** (`bcm2835-sdiod` after the chip ID; as in OpenBSD's bwfm(4)
and Linux' brcmfmac, both ISC, no code taken):

1. Block sizes: function 1 64 bytes, function 2 512 bytes (FBR `0x110`,
   `0x210`).
2. Cores: ChipCommon register `0xfc` points to the enumeration ROM; it
   lists each core with its register and wrapper address. Needed: ARM CR4
   (`0x83e`), 802.11 (`0x812`), SDIO device (`0x829`).
3. RAM: the CR4's bank registers give the size (expected 800 KiB); the base
   is fixed per chip, `0x198000` for the 43455.
4. Hold the ARM (wrapper IOCTL CPUHALT), reset the 802.11 core with PHY
   reset and clock.
5. Write the firmware at the RAM base with CMD53 in 64-byte blocks through
   the 32 KiB backplane window; read it back. At 25 MHz, 4 bits, PIO.
6. NVRAM: the text file reduced to `key=value` entries with NULs, padded,
   plus a length word; written at the very end of RAM.
7. Write the firmware's first word (its reset vector) to address 0,
   release the ARM.
8. Request the HT clock (CHIPCLKCSR `0x10`, wait for `0x80`), send the SDPCM
   protocol version 4 through the to-chip mailbox, enable function 2 and
   wait until it is ready: the firmware runs.
9. Control channel (BCDC over SDPCM on function 2): a frame is a 4-byte
   length header, an 8-byte SDPCM header (sequence, channel 0 = control,
   data offset, the firmware's credit) and a 16-byte BCDC header (command,
   length, request ID). `WLC_GET_VAR` (262) with `ver` gives the firmware
   version string, with `cur_etheraddr` the MAC address. Replies are found
   by polling the SDIO core's interrupt status for "frame indication" and
   reading function 2 (64 bytes first, then the rest).

**Pass on the Pi:** the log shows the cores, `firmware written … and read
back`, `HT clock on`, `function 2 ready`, a version line like
`wl0: … version 7.45.…` and a MAC address `b8:27:eb:…` (or `dc:a6:32:…`).

**Open questions the Pi test answers:**

- The NVRAM file has a fixed `macaddr=b8:27:eb:74:f2:6c`. Linux shows a
  different address per Pi, so the chip's OTP probably wins; the MAC line
  tells.
- Left out for now, added if the firmware does not come up: the
  save/restore and "keep SDIO on" (KSO) setup brcmfmac does for chips with
  save/restore, the 43455's F2 watermark and MES busy control values, and
  the CLM blob (only needed before a scan, step 3).
- Speed: PIO register writes have a spacing delay; the download time is
  printed. 50 MHz high-speed mode later.

### Steps 3 and 4: scan and join (base `0033`, written, not run)

After `ver` and `cur_etheraddr` the probe goes on:

1. **CLM blob:** `clmload` in chunks of 1400 bytes (12-byte header: flags
   with handler version 1, begin/end, type 2 = CLM, length, CRC unused),
   then `clmload_status` must be 0. Firmware 7.45 scans no channels
   without it.
2. **Events and radio:** read `event_msgs` (18 bytes), add SET_SSID, JOIN,
   AUTH, DEAUTH(_IND), ASSOC, DISASSOC(_IND), LINK, PSK_SUP, ESCAN_RESULT,
   write it back; `roam_off` 1 like Pi OS; `WLC_UP`.
3. **Scan:** `escan` (version 1, start, any SSID/BSSID, active, firmware
   default times, all channels). ESCAN_RESULT events with status 8 carry
   BSS descriptions; status 0 ends the scan. The log lists BSSID, channel,
   dBm, security (RSN IE = WPA2, WPA vendor IE, privacy bit = WEP) and name.
   The BSS description is parsed in its naturally aligned layout, with a
   packed layout as fallback (the sources disagree; the Pi test shows
   which one the firmware sends).
4. **Join** (only with `/etc/net/wifi`, `ssid=` and `psk=`): infrastructure
   mode, open authentication, AES, WPA2-PSK, `sup_wpa` 1, the passphrase
   with `WLC_SET_WSEC_PMK` (the firmware derives the key and does the
   4-way handshake), then `WLC_SET_SSID`. Success is LINK with the up
   flag and PSK_SUP status 6 (keyed); DEAUTH, a failed SET_SSID or another
   PSK_SUP status end it with the reason in the log. `sup_wpa` failing
   means the firmware has no supplicant (the standard file).

Events and data frames: channel 1 and 2 payloads start with a 4-byte BCDC
data header, then an Ethernet frame; events have ethertype 0x886c, a
Broadcom header and a 48-byte big-endian event message.

Pi test: the scan needs no person (it should list the Fritz!Box). The join
needs the network name and passphrase on the Pi, so it waits for the
maintainer.

### Step 5: network interface, needs a netstack change first

Sending and receiving Ethernet frames is ready in the frame code (data
header on channel 2). What blocks `network.wlan0`:

- `netstack` looks for `network.*` schemes once, when it starts
  (`10_smolnetd.service`), and takes the first one. A Wi-Fi driver that
  joins after boot is never seen, and with the LAN7800 present `netstack`
  would not take Wi-Fi anyway.
- `daemon::Daemon` needs init's notify pipe, so the probe could not
  run by hand any more; the driver should split into the boot daemon and
  a small tool (scan, join) that talks to it.

So the order is: `netstack` with several adapters (or at least picking up
adapters that appear later, plus a setting for which one to use), then
the daemon with `network.wlan0`, then `dhcpd` on it.

## Next

Milestones, each a Pi test:

1. After #6: the SDIO driver enumerates the chip and reads chip ID 0x4345.
2. Firmware (minimal file) and NVRAM loaded, the "ver" iovar answers;
   `sup_wpa` reads 1.
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
  (firmware files, `LICENCE.cypress`, `debian/config/defines.toml` for the
  standard/minimal choice, `debian/rpi-brcmfmac.conf`)
- [OpenBSD bwfm(4)](https://man.openbsd.org/bwfm.4), `sys/dev/sdmmc/if_bwfm_sdio.c`,
  `sys/dev/ic/bwfm.c` (ISC)
- Linux `drivers/net/wireless/broadcom/brcm80211/brcmfmac/` (ISC), feature
  detection in `feature.c` (`sup_wpa`, `sae`)
- SD Association, SDIO Simplified Specification (CMD5, CMD52, CMD53)
- Upstream Redox `base`: `netstack/src/main.rs` (one adapter only),
  `drivers/storage/bcm2835-sdhcid` (sizes counted 2026-10-29)
