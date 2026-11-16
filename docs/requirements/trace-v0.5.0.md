# Trace: v0.5.0

Status: **draft**, 2026-11-16. Owner: the Requirements agent.

Requirement → issue → test → state for the release **v0.5.0** (published
2026-11-15), build `v0.5.0-dev-214-g600bd997+x59bc`, the same build as
v0.5.0-rc1. This is the release's baseline ([RMP](../process/rmp.md#changes)).
States and the meaning of "row" are in the [README](README.md#trace). The
evidence is in the
[v0.5.0 checklist](https://github.com/redox-rpi/redox-rpi/blob/release-v0.5.0/docs/releases/v0.5.0-checklist.md)
and the pi-tests reports:

- [row 414](https://github.com/redox-rpi/pi-tests/blob/main/reports/414.md):
  rc1 automated, Pi 3B+. Part A on the netboot image with devshd, part B
  on the release-netboot image without devshd.
- [rc1 person test](https://github.com/redox-rpi/pi-tests/blob/main/reports/v0.5.0-rc1-person.md):
  rc1 SD card (U-Boot card), Pi 3B+, the maintainer.
- [row 428](https://github.com/redox-rpi/pi-tests/blob/main/reports/428.md):
  one boot of the v0.5.0 release netboot image, Pi 3B+.
- Checklist: QEMU regression set (`--starts 300`), pwm-audio QEMU check
  26/26, image inspection (REQ-REL-1), size check (REQ-REL-5).

**No test of the v0.5.0 build ran on the Zero 2 W.** The Zero 2 W rc1
netboot image (`+x8882`) was built but has no row; row 415 tested a
direct-boot dev card (#324). Requirements that name the Zero 2 W are
therefore "earlier" (row 286, v0.4.0).

| Requirement | Prio | Level | Issues | Test | State |
|---|---|---|---|---|---|
| [REQ-BOOT-1](boot.md#req-boot-1-boot-from-the-sd-card) SD card boot | must | release | #6, #243, #324 | person test (3B+) | verified (3B+); Zero 2 W earlier (row 286) |
| [REQ-BOOT-2](boot.md#req-boot-2-network-boot) network boot | must | test image | #243 | rows 414, 428 | verified |
| [REQ-BOOT-3](boot.md#req-boot-3-clean-boot-log) clean boot log | should | release | PR #77 | row 414 (no `pcid`/`rtcd`, no panic) | verified |
| [REQ-BOOT-4](boot.md#req-boot-4-boot-time) boot time | should | release | #47 | person test, row 414 | verified (1: 28.3 s); **over the proposed number** (2: 12.0 s, proposed 10 s) |
| [REQ-BOOT-5](boot.md#req-boot-5-damaged-data-partition) damaged data partition | should | release | #232 | row 270 | earlier; **gap** in checklist |
| [REQ-CON-1](console.md#req-con-1-hdmi-console-with-login) HDMI console | must | release | — | person test | verified (1, 2); not shown (3: terminal switch) |
| [REQ-CON-2](console.md#req-con-2-serial-console) serial console | must/should | release | #28 | row 414 (20 of 20) | verified (1, 2) |
| [REQ-USB-1](usb.md#req-usb-1-usb-keyboard) USB keyboard | must | release | #29 | person test | verified (3B+) |
| [REQ-USB-2](usb.md#req-usb-2-usb-hotplug) hotplug | should | release | #17 | row 286 | earlier; **gap** in checklist |
| [REQ-USB-3](usb.md#req-usb-3-usb-mouse) USB mouse | should | release | #48, PR #223 | row 286 | earlier; **gap** in checklist |
| [REQ-USB-4](usb.md#req-usb-4-low-idle-load-of-the-usb-driver) USB idle load | should | release | #29, #214, #104, #344, #373 | row 5 | earlier; **gap** in checklist |
| [REQ-NET-1](network.md#req-net-1-dhcp-on-the-onboard-ethernet) onboard Ethernet | must | release | #217, #92 | row 414 | verified (1, 2); **gap** (3, 4: #259, nightly records #369) |
| [REQ-NET-2](network.md#req-net-2-usb-ethernet-adapters) USB Ethernet | must/should | release | base 0020 | row 286 | earlier (no Zero 2 W or adapter row on v0.5.0) |
| [REQ-NET-3](network.md#req-net-3-several-adapters-at-once) several adapters | should | release | #53, PR #81 | row 286 | earlier |
| [REQ-NET-4](network.md#req-net-4-long-tcp-transfers) long TCP transfers | must | release | #157, #88, #184 | row 414 (100 MiB curl and nc) | verified (1); **gap** (2: sockets freed; FIN fix #184 not tested) |
| [REQ-NET-5](network.md#req-net-5-ipv4-multicast) IPv4 multicast | should | release | #27, PR #194, #216 | row 218 | earlier; **gap** in checklist |
| [REQ-TIME-1](time.md#req-time-1-clock-from-ntp) NTP | must | release | #22 | row 414 (1 s off), person test | verified (1); earlier (2: row 286) |
| [REQ-TIME-2](time.md#req-time-2-clock-without-network) clock without network | should | release | — | QEMU offline, agreed 2026-11-08 | **gap** (planned since v0.4.0, not run) |
| [REQ-TIME-3](time.md#req-time-3-set-the-clock-by-hand) set the clock | should | release | #51, PR #98 | Pi row agreed 2026-11-08 | **gap** (planned since v0.4.0, not run) |
| [REQ-SD-1](sdcard.md#req-sd-1-reliable-reads-and-writes) reliable reads and writes | must | release | #6, #31 | row 414 (sd-stress), person test | verified |
| [REQ-SD-2](sdcard.md#req-sd-2-small-writes) small writes | should | release | #16, #155, PR #73 | row 414 (1 MiB in 0.20 s) | verified (1); **gap** (2) |
| [REQ-SD-3](sdcard.md#req-sd-3-rename-on-every-redoxfs) rename | should | release | #57 | row 83 | earlier; **gap** in checklist |
| [REQ-GPIO-1](gpio.md#req-gpio-1-gpio-pins) GPIO | should | release | #38 | row 414 (no wiring, 21 of 21) | verified (without wiring); **gap** (wired) |
| [REQ-GPIO-2](gpio.md#req-gpio-2-i2c-bus-1) I2C | should | release | #39 | row 414 | verified (without wiring); **gap** (device) |
| [REQ-GPIO-3](gpio.md#req-gpio-3-spi0) SPI | should | release | #42 | row 414 | verified (without wiring); **gap** (loopback) |
| [REQ-FW-1](firmware.md#req-fw-1-full-cpu-speed-without-throttling) 1400 MHz, no throttling | must | release | #20 | row 414 | verified (1); 2 with a note: 4 soft temperature warnings (59–60 °C, no heatsink), ARM stayed at 1400 MHz, released with it |
| [REQ-FW-2](firmware.md#req-fw-2-cpu-clock-follows-the-load) governor | should | release | #23, PR #147 | row 414 (600/1400 MHz) | verified (1); earlier (2: row 161) |
| [REQ-FW-3](firmware.md#req-fw-3-reboot) reboot | must | release | #12 | row 414 (5 of 5), person test | verified |
| [REQ-FW-4](firmware.md#req-fw-4-firmware-information) firmware information | should | release | base 0018 | row 414 (throttled flags read) | verified (3); **gap** (1, 2: board and temperature check) |
| [REQ-AUDIO-1](audio.md#req-audio-1-sound-from-the-headphone-jack-beta) headphone jack | should | beta | #250, #292, #336 | row 414 (44 107 Hz, measured 43 784), QEMU 26/26 | verified |
| [REQ-AUDIO-2](audio.md#req-audio-2-audible-on-both-channels-beta) audible, left/right | should | beta | #250 | person test (startup sound, Sopwith with sound) | verified (heard); earlier (2: left only, row 286); 3: no click at reboot |
| [REQ-GFX-1](graphics.md#req-gfx-1-optional-desktop-from-the-console-beta) optional desktop | should | beta | #33, #298 | row 414, person test | verified (1, 2); not shown (3, 4) |
| [REQ-GFX-2](graphics.md#req-gfx-2-desktop-apps-beta) desktop apps | may | beta | #33, #298 | — | **requirement out of date**: the editor was replaced by Sopwith (#298); 1 and 3 not shown |
| [REQ-TCC-1](tcc.md#req-tcc-1-compile-and-run-c-on-the-pi-experiment) tcc | may | experiment → **beta in the release** | #247, #293, #297 | row 414 (`tcc -run`, tcc script) | verified (2); not shown (1: `tcc -o`); level needs an update |
| [REQ-STAB-1](stability.md#req-stab-1-programs-start-every-time) program starts | must | release | #1, #12, #127 | row 414 (0 of 1000), row 428 (0 of 300) | verified |
| [REQ-STAB-2](stability.md#req-stab-2-pipes-keep-all-data) pipes | must | release | #158 | row 414 (16 MiB) | verified |
| [REQ-STAB-3](stability.md#req-stab-3-c-library-regressions) relibc tests | must | release | #13, #108 | QEMU regression set | verified |
| [REQ-BOARD-1](boards.md#req-board-1-pi-zero-2-w-with-the-same-image) Zero 2 W | should | release | #8, #324 | row 286 (v0.4.0); row 415 (dev card) | **earlier**; no test on the v0.5.0 build |
| [REQ-REL-1](release.md#req-rel-1-no-remote-shell-in-the-release-image) no devshd | must | release | release-process, #367 | image inspection; row 414 part B | verified (1: card partition; 2: on the release-netboot image, port 2323 refused, no open port) |
| [REQ-REL-2](release.md#req-rel-2-the-image-names-its-release-and-build) release name and build | must | release | #237 | rows 414, 428, person test | verified |
| [REQ-REL-3](release.md#req-rel-3-reproducible-release-build) reproducible build | must | release | #257 | row 414 (manifest), release rebuild | verified (1, 2); 3 in part (QEMU boot test green, run 43599887384) |
| [REQ-REL-4](release.md#req-rel-4-published-and-checkable-download) download | must | release | #237 | checklist (`SHA256SUMS`) | verified |
| [REQ-REL-5](release.md#req-rel-5-fits-a-2-gb-card) 2 GB card | should | release | #24 | size check (1 105 199 104 bytes) | verified |

## Summary

- **43 requirements**, unchanged since v0.4.0.
- **Verified on the v0.5.0 build:** 31, 16 of them only in part; 9 only
  earlier, 2 not run, 1 out of date (REQ-GFX-2).
- **Fails:** none. The two v0.4.0 fails are gone: REQ-CON-2 passed 20 of
  20 (#28), and REQ-GFX-2's editor is no longer in the image.
- **Better than v0.4.0:** REQ-CON-2 (fast serial input), REQ-REL-1
  criterion 1 (image inspection), REQ-REL-5 (size check), REQ-FW-4
  criterion 3, REQ-REL-3 criterion 3 in part.
- **Only an earlier row:** REQ-BOARD-1 and REQ-NET-2, REQ-NET-3 (Zero 2 W
  and adapters), REQ-USB-2, REQ-USB-3, REQ-USB-4, REQ-BOOT-5, REQ-NET-5,
  REQ-SD-3.
- **Still not run:** REQ-TIME-2, REQ-TIME-3 (agreed with V&V on 2026-11-08).
- **Must requirements with an untested criterion:** REQ-NET-1 (3, 4),
  REQ-NET-4 (2), REQ-CON-1 (3). Under the RMP, the release checklist
  needs the maintainer's OK for such a gap; the v0.5.0 checklist does not
  name them.

## Requirement updates for v0.6.0

What v0.5.0 changed in the system and the requirement files do not say yet:

- **REQ-GFX-2:** the text editor is gone (#298). New criteria: Sopwith
  starts and plays (with sound on the 3B+, #335), orbterm, Files.
- **REQ-TCC-1:** tcc is in the release image as beta (#297). Level
  experiment → beta; `-static` stays a known limit.
- **Audio:** startup sound and `volume` (#292, #336), and the fade-out at
  reboot and power-off. Candidate new requirement REQ-AUDIO-3, or criteria
  in REQ-AUDIO-1/2 (REQ-AUDIO-2 criterion 3, the click, can become a pass
  criterion).
- **Network:** name server and local domain from DHCP (#290), unknown
  name fails at once (PR #381). Candidate new requirement REQ-NET-7 (DNS);
  REQ-NET-6 is reserved by the parked RFC 0003.
- **REQ-FW-3:** `poweroff` exists but leaves the Pi on (#418). The parked
  REQ-FW-5 covers power off; until then a known issue.
- **REQ-BOOT-4:** the maintainer decides the proposed numbers (crit 2 is
  12.0 s against a proposed 10 s).

## Findings for the other roles

- **V&V:** no row of the v0.5.0 build on the Zero 2 W. For v0.6.0, the
  candidate needs a Zero 2 W row (REQ-BOARD-1, REQ-NET-2), and the
  REQ-TIME-2/3 checks that are agreed but not run.
- **Release & CI:** the release checklist should list the must criteria
  without a test for the maintainer's OK (RMP).
- **Quality:** REQ-FW-1 criterion 2 passed with a heat note; a heatsink
  on the test Pi would make the criterion clean.
- **Maintainer:** decide the proposed numbers (boot time, NTP, audio rate,
  desktop login).
