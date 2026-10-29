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

It prints lines starting with `bcm2835-sdiod:` and ends with
`bcm2835-sdiod: OK` (chip `0x4345` read the same three times) or
`bcm2835-sdiod: FAILED` and the reason. No network interface yet, no
firmware download.

In QEMU the Arasan has no card behind it, so the probe ends with a CMD5
timeout.

Build: `--experiment wifi-sdio` on top of branch `sdhost-ctxleak`.
