# Wi-Fi over SDIO, first probe (experiment `wifi-sdio`)

Groundwork for Wi-Fi on the Pi 3B+ (issue #26, plan in
`docs/topics/wifi.md`). Needs the sdhost SD driver (base `0025`-`0028`),
which frees the Arasan controller for the Wi-Fi chip.

- base `0030`: `rpi-firmwared` gets `/scheme/rpi-firmware/wifi_power`, the
  chip's WL_REG_ON line on the firmware GPIO expander (pin 129 on the 3B+).
- base `0031`: `bcm2835-sdiod`, a probe in `/usr/lib/drivers`, started by
  `50_bcm2835-sdiod.service` on the 3B+. It refuses to run when GPIO 48-53
  are ALT3 (SD card on the Arasan), then:
  - switches WL_REG_ON off and on,
  - puts GPIO 34-39 on ALT3 with pull-ups on 35-39,
  - resets the Arasan at 400 kHz, CMD0, CMD5, CMD3, CMD7,
  - reads the CCCR and the CIS vendor/device (expect vendor `0x02d0`),
  - enables function 1, requests the ALP clock,
  - reads the ChipCommon chip ID at `0x18000000` with CMD52 (bytes) and
    CMD53 (4 bytes), then again with CMD53 at 25 MHz on 4 data lines.

- base `0032`: after the chip ID (`bcm2835-sdiod: chip ID OK`) it
  - lists the backplane cores from the enumeration ROM and reads the
    ARM CR4's RAM size,
  - holds the ARM, writes firmware (the minimal one; `bcm2835-sdiod
    standard` for the other) and the NVRAM block, reads both back,
  - starts the ARM, waits for the HT clock and function 2,
  - asks the firmware over the SDPCM/BCDC control channel for `ver` and
    `cur_etheraddr`.
- base `0033`: then it loads the regulatory data (CLM blob), brings the
  radio up, scans and lists the networks (BSSID, channel, dBm, security,
  name). With `/etc/net/wifi`:

  ```
  ssid=<network name>
  psk=<passphrase, 8 to 63 characters; leave the line out for an open network>
  ```

  it joins that network, WPA2-PSK through the firmware's own supplicant
  (minimal firmware only), and waits for `link up` and `WPA2 keys in place`.
- base `0034` (branch `wifi-join`, on top of netstack-multi #81): after
  the join the driver stays and registers `network.wlan0`. netstack takes
  it as `wlan0`, and dhcpd gets a lease on it (the default route stays on
  `eth0` while the cable has a lease). Ethernet frames go out and come in
  on SDPCM channel 2. The driver polls the chip every 2 ms while idle (the
  interrupt is not wired up yet) and logs `N frames in, M out, …` once a
  minute when the counters changed.
  - The firmware supplicant gets the PMK as 64 hex digits, as from Linux'
    brcmfmac. The driver computes it from the passphrase with
    PBKDF2-HMAC-SHA1 (`psk.rs`, tested with the IEEE 802.11 annex J
    vectors), so the chip's ARM need not.
  - `/etc/net/wifi` also takes `ssid_hex=` and `psk_hex=`.
  - [`wifi-config.py`](wifi-config.py) `<host> 2323 <file>` writes a
    settings file to a running Pi through devshd in that hex form and
    prints only byte counts. The file stays on the build host, outside the
    repository: for the Pi tests `~/.config/redox-rpi/wifi` on homenodellm,
    which the user creates. **Network names and passphrases are never
    committed.**
  - Not yet: rejoining after the link drops, the chip's interrupt,
    scanning from netstack or a settings tool.
- recipe `wifi-firmware` (in `recipes/`, added by `config.toml`):
  downloads firmware, CLM blob, NVRAM file and `LICENCE.cypress` from
  pinned commits of RPi-Distro/firmware-nonfree and linux-firmware, checks
  their SHA-256 and installs them to `/usr/lib/firmware/brcm/`. The files
  are under the Cypress licence (redistribution in binary form, for
  Cypress chips only, no changes); they never go into this repository.

It prints lines starting with `bcm2835-sdiod:` and ends with
`bcm2835-sdiod: OK` or `bcm2835-sdiod: FAILED` and the reason. Without the
firmware files it stops after the chip ID (still `OK`). With
`/etc/net/wifi` it prints `OK` after the join and keeps running.

In QEMU the Arasan has no card behind it, so the probe ends with a CMD5
timeout.

Build: `--experiment wifi-sdio` on top of branch `sdhost-ctxleak`. The
recipe needs network access on the build host (curl).

Pi tests: #57 (chip ID, image from commit `fb78c4a`, before `0032`),
#60 (firmware start, `7d69076`, before `0033`), then scan (see
`docs/test-plan.md`).
