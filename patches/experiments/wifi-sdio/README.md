# Wi-Fi over SDIO (experiment `wifi-sdio`)

Wi-Fi on the Pi 3B+ (CYW43455) and the Pi Zero 2 W (CYW43436 family),
issues [#26](https://github.com/redox-rpi/redox-rpi/issues/26) and
[#783](https://github.com/redox-rpi/redox-rpi/issues/783). Both boards wire
the chip to GPIO 34-39; ALT3 connects them to the Arasan controller, which
is free because the SD card runs on sdhost (default since base `0027`).

```bash
scripts/build-image.sh --experiment wifi-sdio            # Pi 3B+
scripts/build-image.sh --zero2w --experiment wifi-sdio   # Pi Zero 2 W
```

- base `0110`: `rpi-firmwared` gets `/scheme/rpi-firmware/wifi_power`, the
  3B+ chip's WL_REG_ON line on the firmware GPIO expander (pin 129).
- base `0111`: `bcm2835-sdiod` in `/usr/lib/drivers`, started by
  `50_bcm2835-sdiod.service`. It refuses to run when GPIO 48-53 are ALT3
  (SD card on the Arasan), then:
  - switches WL_REG_ON off and on,
  - puts GPIO 34-39 on ALT3 with pull-ups on 35-39,
  - resets the Arasan at 400 kHz, CMD0, CMD5, CMD3, CMD7,
  - reads the CCCR and the CIS vendor/device (expect vendor `0x02d0`),
  - enables function 1, requests the ALP clock,
  - reads the ChipCommon chip ID at `0x18000000` with CMD52 (bytes) and
    CMD53 (4 bytes), then again with CMD53 at 25 MHz on 4 data lines.
- base `0112`: after the chip ID it lists the backplane cores from the
  enumeration ROM, holds the ARM, writes firmware and NVRAM, reads both
  back, starts the ARM, waits for the HT clock and function 2, and asks the
  firmware over the SDPCM/BCDC control channel for `ver` and
  `cur_etheraddr`.
- base `0113`: then it loads the regulatory data (CLM blob), brings the
  radio up, scans and lists the networks (BSSID, channel, dBm, security,
  name). With `/etc/net/wifi`:

  ```
  ssid=<network name>
  psk=<passphrase, 8 to 63 characters; leave the line out for an open network>
  ```

  it joins that network, WPA2-PSK through the firmware's own supplicant,
  and waits for `link up` and `WPA2 keys in place`.
- base `0114`: after the join the driver stays and registers
  `network.wlan0`. netstack takes it as `wlan0`, and dhcpd gets a lease on
  it (the image's `/etc/net/config.toml` has `[interface.wlan0]`; the
  default route stays on `eth0` while the cable has a lease). Ethernet
  frames go out and come in on SDPCM channel 2. The driver polls the chip
  every 2 ms while idle (the interrupt is not wired up yet) and logs
  `N frames in, M out, …` once a minute when the counters changed.
  - The firmware supplicant gets the PMK as 64 hex digits, as from Linux'
    brcmfmac. The driver computes it from the passphrase with
    PBKDF2-HMAC-SHA1 (`psk.rs`, tested with the IEEE 802.11 annex J
    vectors).
  - `/etc/net/wifi` also takes `ssid_hex=` and `psk_hex=`.
- base `0115`: the Pi Zero 2 W.
  - The board comes from the devicetree. WL_REG_ON is GPIO 41; the chip's
    32.768 kHz sleep clock is GPCLK2 on GPIO 43, which the driver starts
    from the 19.2 MHz oscillator if the firmware did not.
  - The CYW43430/43436 run their firmware on an ARM Cortex-M3 with RAM in
    a SOCRAM core at backplane address 0, not on the 43455's Cortex-R4;
    the download and start follow brcmfmac (ISC).
  - Firmware, NVRAM and CLM by chip ID and revision, as Raspberry Pi OS
    links them for the Zero 2 W: chip `0xaa6c` and `0xa9a6` revision 2 or
    later take `brcmfmac43436-sdio.*`, `0xa9a6` revision 1 takes
    `brcmfmac43436s-sdio.*` (no CLM file; the firmware has its own data).
    `bcm2835-sdiod fw=<name>` loads other files, `bcm2835-sdiod standard`
    the 43455's standard firmware instead of the minimal one.
  - It asks the firmware for `sup_wpa` and prints whether the firmware has
    its own supplicant (the 43436S firmware's build tag says `idsup`; for
    the 43436 firmware only the chip can tell).
  - `wlan0` reports its link state: up after the join, then as the
    firmware's link events say, so netstack and dhcpd (#777) follow it.
- recipe `wifi-firmware` (`tools/wifi-firmware/`): downloads the firmware,
  CLM and NVRAM files and the Cypress and Broadcom licences from pinned
  commits of RPi-Distro/firmware-nonfree and linux-firmware, checks their
  SHA-256 and installs them to `/usr/lib/firmware/brcm/`. The files may be
  redistributed in binary form only, unchanged; they never go into this
  repository.
- `wifi-set` (`tools/wifi-set/`): asks for the network name and the
  passphrase with the terminal echo off and writes `/etc/net/wifi` (mode
  600, hex lines). It prints only the lengths.

It prints lines starting with `bcm2835-sdiod:` and ends with
`bcm2835-sdiod: OK` or `bcm2835-sdiod: FAILED` and the reason. Without the
firmware files it stops after the chip ID (still `OK`). With
`/etc/net/wifi` it prints `OK` after the join and keeps running. In QEMU
the Arasan has no card behind it, so the probe ends with a CMD5 timeout.

## Settings for the Pi tests

Network names and passphrases are never committed or printed. For the Pi
tests the settings stay in `~/.config/redox-rpi/wifi` on the build host
(mode 600, `ssid=` and `psk=` lines), which the user creates.
[`wifi-config.py`](wifi-config.py) writes them to a running Pi and prints
only byte counts:

- over devshd: `wifi-config.py <host> 2323 ~/.config/redox-rpi/wifi`;
- over the serial console, with a root shell at its prompt:
  `wifi-config.py --serial <tty> <serial log> ~/.config/redox-rpi/wifi`.
  It runs `wifi-set` and sends each value only after its prompt appears in
  the serial log (the logger keeps reading the console; the script writes
  to the tty and reads the log). At the end it checks that neither value
  reached the log.

## On the Pi

Pi 3B+ (branch `wifi`/`wifi-sdio`, 2026-10-29 to 2026-11-04): chip ID
`0x4345` rev 6 over CMD52/CMD53 (Test 57), firmware 7.45.241 and 7.45.265
started with version and MAC (Test 60), CLM loaded, radio up and 10-15
networks scanned (Tests 77, 91). The join (Test 110) has not run yet.

Pi Zero 2 W: not run yet.

Not yet: rejoining after the link drops, the chip's interrupt, scanning
from netstack or a settings tool, a host-side WPA2 handshake for firmware
without its own supplicant.
