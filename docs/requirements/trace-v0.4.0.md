# Trace: v0.4.0

Requirement → issue → test → state for the release **v0.4.0** (published
2026-11-09), build `v0.4.0-dev-809-g5723a434+x4aaf`, the same build as
v0.4.0-rc2. This is the release's baseline ([RMP](../process/rmp.md#changes)).
States and the meaning of "row" are in the [README](README.md#trace).
Checklist tests are those of
[v0.4.0-checklist.md](https://github.com/redox-rpi/redox-rpi/blob/release-v0.4.0/docs/releases/v0.4.0-checklist.md#tests);
the evidence is in [pi-tests releases/v0.4.0.md](https://github.com/redox-rpi/pi-tests/blob/main/releases/v0.4.0.md).

Rows: 292 (rc2 automated, tests 1–5, 11, 13, 14, 16, 18, 20, 22 A), 286
(rc2 visit with the maintainer on the Pi 3B+ and the Zero 2 W: tests 6–9,
17, 19, 21, 22, 23), the release boot of the v0.4.0 image, and 285 (rc1,
same automated set).

| Requirement | Prio | Level | PBS | Issues | Test | State |
|---|---|---|---|---|---|---|
| [REQ-BOOT-1](https://github.com/redox-rpi/redox-rpi/issues/475) SD card boot | must | release | 2.1.3, 2.1.4, 2.3.3 | #6, #243 | checklist 6, row 286 | verified |
| [REQ-BOOT-2](https://github.com/redox-rpi/redox-rpi/issues/476) network boot | must | test image | 2.1.3, 3.4 | #243 | checklist 1, rows 292, 285 | verified |
| [REQ-BOOT-3](https://github.com/redox-rpi/redox-rpi/issues/477) clean boot log | should | release | 2.6.1 | PR #77 | checklist 3, row 292 | verified |
| [REQ-BOOT-4](https://github.com/redox-rpi/redox-rpi/issues/478) boot time | should | release | 2.1, 2.2.1, 2.3.2 | #47 | checklist 5, row 292; row 215 | verified (1); earlier (2: row 67) |
| [REQ-BOOT-5](https://github.com/redox-rpi/redox-rpi/issues/479) damaged data partition | should | release | 2.1.4 | #232 | row 270 | earlier; **gap** in checklist |
| [REQ-CON-1](https://github.com/redox-rpi/redox-rpi/issues/480) HDMI console | must | release | 2.3.6, 2.4.6 | — | checklist 7, row 286 | verified |
| [REQ-CON-2](https://github.com/redox-rpi/redox-rpi/issues/481) serial console | must/should | release | 2.3.1, 2.4.6 | #28 | checklist 11, rows 292, 285 | verified (1); **fails** (2: 19 of 20, #28) |
| [REQ-USB-1](https://github.com/redox-rpi/redox-rpi/issues/510) USB keyboard | must | release | 2.3.2 | #29 | checklist 7, 9, row 286 | verified |
| [REQ-USB-2](https://github.com/redox-rpi/redox-rpi/issues/511) hotplug | should | release | 2.3.2 | #17 | checklist 8, row 286 | verified |
| [REQ-USB-3](https://github.com/redox-rpi/redox-rpi/issues/512) USB mouse | should | release | 2.3.2 | #48, PR #223 | checklist 21, row 286 | verified |
| [REQ-USB-4](https://github.com/redox-rpi/redox-rpi/issues/513) USB idle load | should | release | 2.3.2 | #29, #214, #104 | row 5 | earlier; **gap** in checklist |
| [REQ-NET-1](https://github.com/redox-rpi/redox-rpi/issues/491) onboard Ethernet | must | release | 2.3.4, 2.4.2 | #217, #92 | checklist 1, row 292 | verified (1, 2); **gap** (3, 4: #259) |
| [REQ-NET-2](https://github.com/redox-rpi/redox-rpi/issues/492) USB Ethernet | must/should | release | 2.3.4 | base 0020 | checklist 9, 19, row 286 | verified |
| [REQ-NET-3](https://github.com/redox-rpi/redox-rpi/issues/493) several adapters | should | release | 2.4.2 | #53, PR #81 | checklist 19, row 286 | verified (Zero 2 W hub; both adapters at once not shown) |
| [REQ-NET-4](https://github.com/redox-rpi/redox-rpi/issues/494) long TCP transfers | must | release | 2.4.2, 2.2.1 | #157, #88 | checklist 20, row 292 | verified (1); **gap** (2) |
| [REQ-NET-5](https://github.com/redox-rpi/redox-rpi/issues/495) IPv4 multicast | should | release | 2.3.4, 2.4.2 | #27, PR #194 | row 218 | earlier; **gap** in checklist |
| [REQ-TIME-1](https://github.com/redox-rpi/redox-rpi/issues/508) NTP | must | release | 2.4.4 | #22 | checklist 4, rows 292, 286 | verified (1, and 2 in row 286: cable plugged in later) |
| [REQ-TIME-2](https://github.com/redox-rpi/redox-rpi/issues/509) clock without network | should | release | 2.4.4 | — | QEMU offline, `--data` | planned |
| [REQ-TIME-3](https://github.com/redox-rpi/redox-rpi/issues/51) set the clock | should | release | 2.4.1, 2.4.4 | #51, PR #98 | Pi row (devsh, `su user`); dry run on stable 748 passed | planned |
| [REQ-SD-1](https://github.com/redox-rpi/redox-rpi/issues/501) reliable reads and writes | must | release | 2.3.3, 2.4.3 | #6, #31 | checklist 6, 18, rows 292, 286 | verified |
| [REQ-SD-2](https://github.com/redox-rpi/redox-rpi/issues/502) small writes | should | release | 2.4.3 | #16, #155, PR #73 | checklist 18, row 292 | verified (1); **gap** (2) |
| [REQ-SD-3](https://github.com/redox-rpi/redox-rpi/issues/503) rename | should | release | 2.4.3 | #57 | row 83 | earlier; **gap** in checklist |
| [REQ-GPIO-1](https://github.com/redox-rpi/redox-rpi/issues/486) GPIO | should | release | 2.3.9 | #38 | checklist 14, row 292 | verified (without wiring); **gap** (wired) |
| [REQ-GPIO-2](https://github.com/redox-rpi/redox-rpi/issues/487) I2C | should | release | 2.3.9 | #39 | checklist 14, row 292 | verified (without wiring); **gap** (device) |
| [REQ-GPIO-3](https://github.com/redox-rpi/redox-rpi/issues/488) SPI | should | release | 2.3.9 | #42 | checklist 14, row 292 | verified (without wiring); **gap** (loopback) |
| [REQ-FW-1](https://github.com/redox-rpi/redox-rpi/issues/482) 1400 MHz, no throttling | must | release | 2.3.8 | #20 | checklist 2, row 292 | verified |
| [REQ-FW-2](https://github.com/redox-rpi/redox-rpi/issues/483) governor | should | release | 2.3.8, 2.6.1 | #23, PR #147 | checklist 13, row 292; row 161 | verified (1); earlier (2) |
| [REQ-FW-3](https://github.com/redox-rpi/redox-rpi/issues/484) reboot | must | release | 2.2.1, 2.5.2 | #12, #7 | checklist 5, row 292 | verified |
| [REQ-FW-4](https://github.com/redox-rpi/redox-rpi/issues/485) firmware information | should | release | 2.3.8 | base 0018 | new pi-test check (also nightly) | planned |
| [REQ-AUDIO-1](https://github.com/redox-rpi/redox-rpi/issues/250) headphone jack | should | beta | 2.3.10, 2.4.5 | #250, PR #251 | checklist 16, row 292 (44 107 Hz) | verified |
| [REQ-AUDIO-2](https://github.com/redox-rpi/redox-rpi/issues/472) audible, left/right | should | beta | 2.3.10 | #250 | checklist 17, row 286 | verified (click at power-off and reboot noted) |
| [REQ-GFX-1](https://github.com/redox-rpi/redox-rpi/issues/489) optional desktop | should | beta (rc2) | 2.5.4, 2.3.6 | #33, PR #255 | checklist 22, rows 292 (22 A), 286 | verified |
| [REQ-GFX-2](https://github.com/redox-rpi/redox-rpi/issues/490) desktop apps | may | beta (rc2) | 2.5.4 | #33, PR #255 | checklist 22, row 286 | **fails** (2: the editor cannot save, #33, known issue); verified (1, 3) |
| [REQ-TCC-1](https://github.com/redox-rpi/redox-rpi/issues/507) tcc | may | experiment | 2.5.3 | #247, PR #249 | row 278 | earlier (draft PR) |
| [REQ-STAB-1](https://github.com/redox-rpi/redox-rpi/issues/504) program starts | must | release | 2.2.1, 2.4.1 | #1, #12, #127 | checklist 1, row 292 (0 of 1000) | verified |
| [REQ-STAB-2](https://github.com/redox-rpi/redox-rpi/issues/505) pipes | must | release | 2.2.1 | #158 | checklist 20, row 292 | verified |
| [REQ-STAB-3](https://github.com/redox-rpi/redox-rpi/issues/506) relibc tests | must | release | 2.4.1 | #13, #108 | checklist 15 (QEMU) | verified |
| [REQ-BOARD-1](https://github.com/redox-rpi/redox-rpi/issues/474) Zero 2 W | should | release | 1.2 | #8 | checklist 9, 19, row 286 | verified |
| [REQ-REL-1](https://github.com/redox-rpi/redox-rpi/issues/496) no devshd | must | release | 2.6.1, 3.3 | release-process | checklist 23, row 286 | verified (2: port 2323 closed on both boards, no devshd in `ps`); criterion 1 (image inspection) not run |
| [REQ-REL-2](https://github.com/redox-rpi/redox-rpi/issues/497) release name and build | must | release | 2.6.1, 4.1 | #237 | checklist 1, row 292; release boot | verified |
| [REQ-REL-3](https://github.com/redox-rpi/redox-rpi/issues/498) reproducible build | must | release | 3.1 | #257 | checklist 1, row 292 | verified (1, 2); **gap** (3: CI green, #257) |
| [REQ-REL-4](https://github.com/redox-rpi/redox-rpi/issues/499) download | must | release | 4.1, 3.4 | #237 | checklist 10 | verified |
| [REQ-REL-5](https://github.com/redox-rpi/redox-rpi/issues/500) 2 GB card | should | release | 2.6.1, 4.1 | #24 | Release & CI size check | planned |

## Summary

- **43 requirements:** 19 must (2 of them must/should), 22 should, 2 may;
  37 release, 4 beta, 1 experiment, 1 test image.
- **Verified on the v0.4.0 build:** 33, 12 of them only in part.
- **Fails:** 2 criteria, both known issues in the release notes:
  REQ-CON-2 fast serial input (#28), REQ-GFX-2 editor save (#33).
- **Planned (method agreed, not run):** REQ-TIME-2, REQ-TIME-3, REQ-FW-4,
  REQ-REL-5, and REQ-REL-1 criterion 1.
- **Only an earlier row:** REQ-BOOT-5, REQ-USB-4, REQ-NET-5, REQ-SD-3,
  REQ-TCC-1 (experiment).
- **Criteria without a test** inside verified requirements: REQ-NET-1 (network
  on every boot and for an hour, #259), REQ-NET-4 (sockets freed), REQ-SD-2
  (2 s to the card), REQ-GPIO-1–3 (with wiring), REQ-REL-3 (CI green, #257).
- **Must requirements with an untested criterion:** REQ-NET-1 (3, 4),
  REQ-NET-4 (2), REQ-REL-1 (1), REQ-REL-3 (3). Under the RMP such a gap
  needs the maintainer's OK in the release checklist; v0.4.0 was released
  before that rule existed.

## Findings for the other roles

- **V&V:** the planned checks (REQ-TIME-2 QEMU, REQ-TIME-3 Pi row, REQ-FW-4
  pi-test check) for v0.4.1; rows name REQ IDs (row 286 did).
- **Release & CI:** the image inspection for REQ-REL-1 and the size check
  for REQ-REL-5 as checklist items.
- **Quality:** KPI candidates: criteria without a test (list above), and
  requirements verified only on an earlier build.
- **Maintainer:** numbers marked "(proposed)" in the requirement files
  need a decision: boot time 30 s, NTP within 2 s, network on 100 of 100
  boots, audio rate ±2 %, desktop login within 30 s.
