# Trace: v0.4.0

Requirement → issue → test → state, for v0.4.0-rc1 (build
`v0.4.0-dev-738-g4804fe63+x12cb`) and the desktop planned for rc2. States
and the meaning of "row" are in the [README](README.md#trace). Checklist
tests are those of [v0.4.0-checklist.md](https://github.com/redox-rpi/redox-rpi/blob/release-v0.4.0/docs/releases/v0.4.0-checklist.md#tests)
(branch `release-v0.4.0`).

State on 2026-11-08: row 285 passed; row 286 (the visit) is queued.

| Requirement | Prio | Level | Issues | Test | State |
|---|---|---|---|---|---|
| [REQ-BOOT-1](boot.md#req-boot-1-boot-from-the-sd-card) SD card boot | must | release | #6, #243 | checklist 6, row 286 | queued (earlier: v0.3.0 visit 193) |
| [REQ-BOOT-2](boot.md#req-boot-2-network-boot) network boot | must | test image | #243 | checklist 1, row 285 | verified |
| [REQ-BOOT-3](boot.md#req-boot-3-clean-boot-log) clean boot log | should | release | PR #77 | checklist 3, row 285 | verified |
| [REQ-BOOT-4](boot.md#req-boot-4-boot-time) boot time | should | release | #47 | checklist 5, row 285; row 215 | verified (1); earlier (2: row 67) |
| [REQ-BOOT-5](boot.md#req-boot-5-damaged-data-partition) damaged data partition | should | release | #232 | row 270 | earlier; **gap** in checklist |
| [REQ-CON-1](console.md#req-con-1-hdmi-console-with-login) HDMI console | must | release | — | checklist 7, row 286 | queued (earlier: visit 193) |
| [REQ-CON-2](console.md#req-con-2-serial-console) serial console | must/should | release | #28 | checklist 11, row 285 | verified (1); **fails** (2: 19 of 20, #28) |
| [REQ-USB-1](usb.md#req-usb-1-usb-keyboard) USB keyboard | must | release | #29 | checklist 7, 9, row 286 | queued |
| [REQ-USB-2](usb.md#req-usb-2-usb-hotplug) hotplug | should | release | #17 | checklist 8, row 286; rows 35, 67 | queued |
| [REQ-USB-3](usb.md#req-usb-3-usb-mouse) USB mouse | should | release | #48, PR #223 | checklist 21, row 286 | queued |
| [REQ-USB-4](usb.md#req-usb-4-low-idle-load-of-the-usb-driver) USB idle load | should | release | #29, #214, #104 | row 5 | earlier; **gap** in checklist |
| [REQ-NET-1](network.md#req-net-1-dhcp-on-the-onboard-ethernet) onboard Ethernet | must | release | #217, #92 | checklist 1, row 285 | verified (1, 2); **gap** (3, 4: #259) |
| [REQ-NET-2](network.md#req-net-2-usb-ethernet-adapters) USB Ethernet | must/should | release | base 0020 | checklist 9, 19, row 286 | queued |
| [REQ-NET-3](network.md#req-net-3-several-adapters-at-once) several adapters | should | release | #53, PR #81 | checklist 19, row 286 | queued |
| [REQ-NET-4](network.md#req-net-4-long-tcp-transfers) long TCP transfers | must | release | #157, #88 | checklist 20, row 285 | verified (1); **gap** (2) |
| [REQ-NET-5](network.md#req-net-5-ipv4-multicast) IPv4 multicast | should | release | #27, PR #194 | row 218 | earlier; **gap** in checklist |
| [REQ-TIME-1](time.md#req-time-1-clock-from-ntp) NTP | must | release | #22 | checklist 4, row 285 | verified (1); **gap** (2) |
| [REQ-TIME-2](time.md#req-time-2-clock-without-network) clock without network | should | release | — | — | **gap** |
| [REQ-TIME-3](time.md#req-time-3-set-the-clock-by-hand) set the clock | should | release | #51, PR #98 | — | **gap** |
| [REQ-SD-1](sdcard.md#req-sd-1-reliable-reads-and-writes) reliable reads and writes | must | release | #6, #31 | checklist 6, 18, rows 285, 286 | verified (data partition); queued (root on card) |
| [REQ-SD-2](sdcard.md#req-sd-2-small-writes) small writes | should | release | #16, #155, PR #73 | checklist 18, row 285 | verified (1); **gap** (2) |
| [REQ-SD-3](sdcard.md#req-sd-3-rename-on-every-redoxfs) rename | should | release | #57 | row 83 | earlier; **gap** in checklist |
| [REQ-GPIO-1](gpio.md#req-gpio-1-gpio-pins) GPIO | should | release | #38 | checklist 14, row 285 | verified (without wiring); **gap** (wired) |
| [REQ-GPIO-2](gpio.md#req-gpio-2-i2c-bus-1) I2C | should | release | #39 | checklist 14, row 285 | verified (without wiring); **gap** (device) |
| [REQ-GPIO-3](gpio.md#req-gpio-3-spi0) SPI | should | release | #42 | checklist 14, row 285 | verified (without wiring); **gap** (loopback) |
| [REQ-FW-1](firmware.md#req-fw-1-full-cpu-speed-without-throttling) 1400 MHz, no throttling | must | release | #20 | checklist 2, row 285 | verified |
| [REQ-FW-2](firmware.md#req-fw-2-cpu-clock-follows-the-load) governor | should | release | #23, PR #147 | checklist 13, row 285; row 161 | verified (1); earlier (2) |
| [REQ-FW-3](firmware.md#req-fw-3-reboot) reboot | must | release | #12, #7 | checklist 5, row 285 | verified |
| [REQ-FW-4](firmware.md#req-fw-4-firmware-information) firmware information | should | release | base 0018 | — | **gap** |
| [REQ-AUDIO-1](audio.md#req-audio-1-sound-from-the-headphone-jack-beta) headphone jack | should | beta | #250, PR #251 | checklist 16, row 285 | verified |
| [REQ-AUDIO-2](audio.md#req-audio-2-audible-on-both-channels-beta) audible, left/right | should | beta | #250 | checklist 17, row 286; row 280 | queued (earlier: row 280) |
| [REQ-GFX-1](graphics.md#req-gfx-1-optional-desktop-from-the-console-beta) optional desktop | should | beta (rc2) | #33, PR #255 | checklist 22 (planned); rows 287, 288 | earlier (dirty build); **gap** (automated smoke, #261) |
| [REQ-GFX-2](graphics.md#req-gfx-2-desktop-apps-beta) desktop apps | may | beta (rc2) | #33, PR #255 | checklist 22 (planned); rows 274, 275 | earlier |
| [REQ-TCC-1](tcc.md#req-tcc-1-compile-and-run-c-on-the-pi-experiment) tcc | may | experiment | #247, PR #249 | row 278 | earlier (draft PR) |
| [REQ-STAB-1](stability.md#req-stab-1-programs-start-every-time) program starts | must | release | #1, #12, #127 | checklist 1, row 285 | verified |
| [REQ-STAB-2](stability.md#req-stab-2-pipes-keep-all-data) pipes | must | release | #158 | checklist 20, row 285 | verified |
| [REQ-STAB-3](stability.md#req-stab-3-c-library-regressions) relibc tests | must | release | #13, #108 | checklist 15 (QEMU) | verified |
| [REQ-BOARD-1](boards.md#req-board-1-pi-zero-2-w-with-the-same-image) Zero 2 W | should | release | #8 | checklist 9, 19, row 286 | queued (earlier: visit 193) |
| [REQ-REL-1](release.md#req-rel-1-no-remote-shell-in-the-release-image) no devshd | must | release | release-process | — | **gap** |
| [REQ-REL-2](release.md#req-rel-2-the-image-names-its-release-and-build) release name and build | must | release | #237 | checklist 1, row 285 | verified |
| [REQ-REL-3](release.md#req-rel-3-reproducible-release-build) reproducible build | must | release | #257 | checklist 1, row 285 | verified (1, 2); **gap** (3: CI green, #257) |
| [REQ-REL-4](release.md#req-rel-4-published-and-checkable-download) download | must | release | #237 | checklist 10 | verified |
| [REQ-REL-5](release.md#req-rel-5-fits-a-2-gb-card) 2 GB card | should | release | #24 | — | **gap** |

## Summary

- **43 requirements:** 19 must (2 of them must/should), 22 should, 2 may;
  37 release, 4 beta (2 of them for rc2), 1 experiment, 1 test image.
- **Verified on rc1:** 22 (row 285, checklist 10 and 15), 12 of them only
  in part.
- **Queued:** 9, all in the visit row 286.
- **Fails:** 1 criterion: REQ-CON-2 fast serial input (#28, known issue).
- **No test at all (gap):** 5 — REQ-TIME-2, REQ-TIME-3, REQ-FW-4,
  REQ-REL-1, REQ-REL-5.
- **Only an earlier row, no checklist test:** 7 — REQ-BOOT-5, REQ-USB-4,
  REQ-NET-5, REQ-SD-3, REQ-GFX-1, REQ-GFX-2, REQ-TCC-1 (the last three are
  rc2 or experiment).
- **Criteria not tested on rc1** inside otherwise tested requirements: 11 —
  REQ-NET-1 (network on every boot and for an hour), REQ-NET-4 (sockets
  freed), REQ-TIME-1 (late cable), REQ-SD-2 (2 s to the card), REQ-GPIO-1–3
  (with wiring), REQ-REL-3 (CI green), REQ-GFX-1 (automated smoke test),
  REQ-BOOT-4 (kernel to login on rc1), REQ-FW-2 (governor file on rc1).

## Findings for the other roles

- **V&V:** REQ-REL-1 is a must with no test; one line in the visit row 286
  (`nc <pi> 2323` refused, no `devshd` in `ps`) closes it. REQ-TIME-2 and
  REQ-TIME-3 fit the same visit (boot without cable). Rows should name the
  REQ IDs they verify from the next row on.
- **V&V and Quality:** all automated rows run on the netboot image with
  `devshd`, never on the SD release image itself
  ([README](README.md#trace)).
- **Quality:** the gaps above are the first KPI candidates: number of must
  requirements without a passing test on the candidate (today: REQ-REL-1 no
  test, REQ-CON-2 fails).
- **Project lead and topic owners:** issues that are implemented in rc1 but
  still open or in a later milestone: #38, #39, #42 (v0.8.0), #48 (v0.5.0),
  #53 (v0.6.0), #51, #57, #23, #250. Their owners should close them or say
  what is left.
- **Maintainer:** numbers marked "(proposed)" in the requirement files need
  a decision: boot time 30 s, NTP within 2 s, network on 100 of 100 boots,
  audio rate ±2 %, desktop login within 30 s, image at most 1 900 MB.
