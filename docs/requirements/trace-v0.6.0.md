# Trace: v0.6.0

Status: **draft**, 2026-11-22. Owner: the Requirements agent.

Requirement → issue → test → state for the release **v0.6.0**, candidate
**rc3**, build `v0.6.0-dev-135-g0897391f+x6a9f` (Zero 2 W network boot
`+x1c03`). States and the meaning of "row" are in the
[README](README.md#trace). Prio comes from the issue field (High = must,
Medium = should, Low = may; [RMP](../process/rmp.md)). The evidence is in
the [v0.6.0 checklist](../releases/v0.6.0-checklist.md) and these pi-tests
reports:

| Test | Build | Board | What |
|---|---|---|---|
| [474](https://github.com/redox-rpi/pi-tests/blob/main/reports/474.md) | rc1 `+xc80e` | 3B+ | automated release row (A netboot, B release image, C poweroff); fail: no `libz.so.1` (#608) |
| [450](https://github.com/redox-rpi/pi-tests/blob/main/reports/450.md) | rc1 | 3B+ | A 100 boots (fail 66/100, #576), B 1 h idle (pass), C 1000 connections (fail, memory grows) |
| [458](https://github.com/redox-rpi/pi-tests/blob/main/reports/458.md) | rc1 | 3B+ | tcc set: 2 of 3 runs pass |
| [570](https://github.com/redox-rpi/pi-tests/blob/main/reports/570.md) | rc2 `+xcbcb` | 3B+ | 100 boots: 100 of 100 with network (41.2–42.4 s) |
| [571](https://github.com/redox-rpi/pi-tests/blob/main/reports/571.md) | rc2 | 3B+ | automated release row again (A, B, C) and tcc set (8 of 9 runs pass) |
| [449](https://github.com/redox-rpi/pi-tests/blob/main/reports/449.md) | rc2 | Zero 2 W | person test on the direct card: boot, login, VT switch, memory |
| [573](https://github.com/redox-rpi/pi-tests/blob/main/reports/573.md) | rc3 `+x6a9f` | 3B+ | delta, automated: pi-test, regression set `--starts 300`, 5 reboots, HDMI sound (no sink, `hdmi_mode no`) |
| [574](https://github.com/redox-rpi/pi-tests/blob/main/reports/574.md) | rc3 | Zero 2 W | person test: HDMI sound audible; LAN over a USB Realtek adapter, NTP |
| [575](https://github.com/redox-rpi/pi-tests/blob/main/reports/575.md) | rc3 | 3B+ | person test on the rc3 card: boot without a cable, HDMI login, USB keyboard, cable in after boot (DHCP, NTP), chime and Sopwith tune over HDMI; overscan border (#347/#348) |

Plus the checklist's candidate checks on each card (REQ-REL-1, REQ-REL-5,
no missing shared library) and the QEMU regression set on each netboot
build.

**rc2 rows count for rc3.** rc3 is rc2 plus the experiment `hdmi-audio`
(#432, #613) and the quiet-boot code of #604, which does nothing without
`QUIET`. A row on rc2 counts for rc3 unless the requirement touches audio;
the table says "rc2" where it does. rc1 rows count only where rc2 changed
nothing in the path (#609 libz, #603 USB root port after a warm reboot).

**The direct card was booted by people on both boards:** the 3B+ in
Test 575, the Zero 2 W in Tests 449 and 574. The other 3B+ rows are
automated (network boot and the release-netboot image).

| Requirement | Prio | Level | Issues | Test | State |
|---|---|---|---|---|---|
| [REQ-BOOT-1](https://github.com/redox-rpi/redox-rpi/issues/475) SD card boot | must | release | #6, #243, #324 | 575 (3B+), 449, 574 (Zero 2 W) | 1 verified (575: no cable at boot, login on HDMI); 2, 3: **requirement out of date**: the direct card runs the root filesystem from RAM (#324), so there is no RedoxFS root on the card by design |
| [REQ-BOOT-2](https://github.com/redox-rpi/redox-rpi/issues/476) network boot | must | test image | #243 | 570, 571, 573 | verified |
| [REQ-BOOT-3](https://github.com/redox-rpi/redox-rpi/issues/477) clean boot log | should | release | PR #77 | 571, 573 (no `pcid`/`rtcd`, audiod without error, panic 0) | verified |
| [REQ-BOOT-4](https://github.com/redox-rpi/redox-rpi/issues/478) boot time | should | release | #47 | — | not measured on v0.6.0; earlier (v0.5.0: 1: 28.3 s; 2: 12.0 s, over the proposed 10 s) |
| [REQ-BOOT-5](https://github.com/redox-rpi/redox-rpi/issues/479) damaged data partition | should | release | #232 | row 270 | earlier; **gap** in checklist |
| [REQ-CON-1](https://github.com/redox-rpi/redox-rpi/issues/480) HDMI console | must | release | — | 575 (3B+), 449 (Zero 2 W) | verified (1, 2, 3 on both boards; 3B+: Super+F1/F2/F3, typing after switching back, the maintainer in the session of Test 575); note: black border on the 3B+ at 1920x1080 (overscan, #347/#348) |
| [REQ-CON-2](https://github.com/redox-rpi/redox-rpi/issues/481) serial console | must | release | #28, #547 | 571 (20 of 20, 0 late, 0 lost) | verified (1, 2) |
| [REQ-USB-1](https://github.com/redox-rpi/redox-rpi/issues/510) USB keyboard | must | release | #29 | 575 (3B+), 449 (Zero 2 W) | verified (1, on one port of the 3B+; 2: key repeat, the maintainer in the session of Test 575) |
| [REQ-USB-2](https://github.com/redox-rpi/redox-rpi/issues/511) hotplug | should | release | #17 | 575 (cable after boot), row 286 | 2 verified (575: address after the cable went in; time not recorded); 1, 3 earlier |
| [REQ-USB-3](https://github.com/redox-rpi/redox-rpi/issues/512) USB mouse | should | release | #48, PR #223 | row 286 | earlier; **gap** in checklist |
| [REQ-USB-4](https://github.com/redox-rpi/redox-rpi/issues/513) USB idle load | should | release | #29, #214, #104, #344, #373 | row 5 | earlier; **gap** in checklist |
| [REQ-NET-1](https://github.com/redox-rpi/redox-rpi/issues/491) DHCP address on every supported adapter | must | release | #217, #92, #576 | 570, 571, 574, 450 | 1 verified (3B+ 571; Zero 2 W 574, about 10 s); 4 verified (571); 5 verified on rc2 (570, 100 of 100; rc1 had 66 of 100, #576); 6 verified on rc1 only (450 B; rc2 changed only the USB root port path); 2, 3 **not tested** (new wording 2026-11-22; PR #614 is for v0.7.0) |
| [REQ-NET-2](https://github.com/redox-rpi/redox-rpi/issues/492) supported Ethernet adapters | may | release | base `0020` | 571 (LAN7800), 574 (USB Realtek) | verified (1, 2); new wording approved by the maintainer 2026-11-22 |
| [REQ-NET-3](https://github.com/redox-rpi/redox-rpi/issues/493) several adapters | should | release | #53, PR #81 | row 286 | earlier |
| [REQ-NET-4](https://github.com/redox-rpi/redox-rpi/issues/494) long TCP transfers | must | release | #157, #88, #184 | 571 (curl 100 MiB in 22.2 s, SHA-256 right; nc), 450 C | 1 verified; 2 **fails** on rc1 (450 C: 1000 connections work, but memory grows, about 300 KB per connection, mostly outside any process; no socket count; #88); not run on rc2/rc3 |
| [REQ-NET-5](https://github.com/redox-rpi/redox-rpi/issues/495) IPv4 multicast | should | release | #27, PR #194, #216 | row 218 | earlier; **gap** in checklist |
| [REQ-NET-7](https://github.com/redox-rpi/redox-rpi/issues/471) name lookup (DNS) | must | release | #290, PR #381, base `0088` | 571 (step 10), 573 (regression `dns`) | proposed requirement; 1 verified (full name 0.5 s, short name 0.2 s); 2 verified (regression set `dns`, Pi and QEMU); 3 verified (0.2 s); 4 **not tested** |
| [REQ-TIME-1](https://github.com/redox-rpi/redox-rpi/issues/508) NTP | must | release | #22 | 571 (= host), 574 (Zero 2 W), 575 | verified (1; 2: 575, cable in after boot, clock set from NTP) |
| [REQ-TIME-2](https://github.com/redox-rpi/redox-rpi/issues/509) clock without network | should | release | — | — | **gap** (planned since v0.4.0, not run) |
| [REQ-TIME-3](https://github.com/redox-rpi/redox-rpi/issues/51) set the clock | should | release | #51, PR #98 | — | **gap** (planned since v0.4.0, not run) |
| [REQ-SD-1](https://github.com/redox-rpi/redox-rpi/issues/501) reliable reads and writes | must | release | #6, #31 | 571 (sd-stress, `sdcard`) | verified (on the netboot image's data partition) |
| [REQ-SD-2](https://github.com/redox-rpi/redox-rpi/issues/502) small writes | should | release | #16, #155, PR #73 | 571 (1 MiB in 0.18 s) | verified (1); **gap** (2) |
| [REQ-SD-3](https://github.com/redox-rpi/redox-rpi/issues/503) rename | should | release | #57 | row 83 | earlier; **gap** in checklist |
| [REQ-GPIO-1](https://github.com/redox-rpi/redox-rpi/issues/486) GPIO | should | release | #38 | 571 (no wiring, 21 of 21) | verified (without wiring); **gap** (wired) |
| [REQ-GPIO-2](https://github.com/redox-rpi/redox-rpi/issues/487) I2C | should | release | #39 | 571 | verified (without wiring); **gap** (device) |
| [REQ-GPIO-3](https://github.com/redox-rpi/redox-rpi/issues/488) SPI | should | release | #42 | 571 | verified (without wiring); **gap** (loopback) |
| [REQ-FW-1](https://github.com/redox-rpi/redox-rpi/issues/482) 1400 MHz, no throttling | must | release | #20 | 571 (max 1400 MHz, `throttled 0x00000000`) | verified (1, 2) |
| [REQ-FW-2](https://github.com/redox-rpi/redox-rpi/issues/483) governor | should | release | #23, PR #147 | 571 (`ondemand`, 600/1400 MHz) | verified (1); earlier (2: row 161) |
| [REQ-FW-3](https://github.com/redox-rpi/redox-rpi/issues/484) reboot and power off | must | release | #12, #418, PR #526 | 571 (5 of 5, part C), 573 (5 of 5) | verified (1 on rc3; 2, 3 on rc2: 0.0 W for 60 s, file unchanged) |
| [REQ-FW-4](https://github.com/redox-rpi/redox-rpi/issues/485) firmware information | should | release | base 0018 | 571 (throttled flags) | verified (3); **gap** (1, 2) |
| [REQ-AUDIO-1](https://github.com/redox-rpi/redox-rpi/issues/250) headphone jack | should | beta | #250, #292, #336 | 571 (rc2: 44 088 Hz, tone 3 s) | verified on rc2 only; **known deviation in v0.6.0**: with `hdmi-audio` there is no `bcm2835-pwmaudiod`, the jack is silent; the maintainer accepted this on 2026-11-22, a choice between jack and HDMI follows in #617 (v0.7.0) |
| [REQ-AUDIO-2](https://github.com/redox-rpi/redox-rpi/issues/472) audible, left/right | should | beta | #250 | — | **known deviation in v0.6.0** (jack silent with `hdmi-audio`, accepted 2026-11-22, #617); not heard on v0.6.0 |
| [REQ-AUDIO-3](https://github.com/redox-rpi/redox-rpi/issues/473) startup sound and volume | should | beta | #292, #336 | 571, 573, 574 | proposed requirement; 1 in part (`startup-sound: played` in every boot, 573 on rc3, heard on HDMI in 574; time not measured); 3 verified on rc2 (571: 70 survives the reboots); 2 not tested; 4 goal |
| [REQ-GFX-1](https://github.com/redox-rpi/redox-rpi/issues/489) optional desktop | should | beta | #33, #298 | 571, 573, 449 | verified (1, 2: Orbital starts; on the Zero 2 W only with `sudo desktop`; 3: Super+F2/F3 on the 3B+, Test 575 session); 4 not shown |
| [REQ-GFX-2](https://github.com/redox-rpi/redox-rpi/issues/490) desktop apps | may | beta | #298, #335 | 573 (`launcher-sopwith`), 575 (tune heard), QEMU desktop check, 449 | proposed requirement; 2 verified (575, 3B+: started from the launcher, controls per the in-game help, tune heard over HDMI; criterion reworded 2026-11-22, Sopwith's default keys are not the arrow keys); 1, 3 not shown |
| [REQ-TCC-1](https://github.com/redox-rpi/redox-rpi/issues/507) tcc | may | beta | #247, #293, #297 | 571, 573, 458 | proposed requirement; 1, 2, 4 verified; 5 verified with a note: 573 pass, 571 8 of 9 runs, 458 2 of 3 runs; one step fails rarely, cause unknown |
| [REQ-STAB-1](https://github.com/redox-rpi/redox-rpi/issues/504) program starts | must | release | #1, #12, #127 | 571 (0 of 1000), 573 (0 of 300) | verified |
| [REQ-STAB-2](https://github.com/redox-rpi/redox-rpi/issues/505) pipes | must | release | #158 | 571 (16 MiB) | verified |
| [REQ-STAB-3](https://github.com/redox-rpi/redox-rpi/issues/506) relibc tests | must | release | #13, #108 | QEMU regression set (rc1, rc2, rc3), 573 | verified (as on v0.5.0, by the regression set) |
| [REQ-BOARD-1](https://github.com/redox-rpi/redox-rpi/issues/474) Zero 2 W | should | release | #8, #324 | 449 (rc2), 574 (rc3) | verified (1, 2) |
| [REQ-REL-1](https://github.com/redox-rpi/redox-rpi/issues/496) no devshd | must | release | release-process, #367 | rc3 card inspection; 571 part B (rc2) | verified (1 on rc3; 2 on rc2: port 2323 refused, no open port) |
| [REQ-REL-2](https://github.com/redox-rpi/redox-rpi/issues/497) release name and build | must | release | #237 | 571, 573, 574 | verified (1, 2); 3 not shown |
| [REQ-REL-3](https://github.com/redox-rpi/redox-rpi/issues/498) reproducible build | must | release | #257 | 573 (`--manifest`), manifests `dirty = "no"` | verified (1, 2); 3 in part (QEMU boot test green on the rc1 commit; rc3 commit not recorded) |
| [REQ-REL-4](https://github.com/redox-rpi/redox-rpi/issues/499) download | must | release | #237 | — | at release (checklist: `SHA256SUMS`); the rc3 card has its sha256 |
| [REQ-REL-5](https://github.com/redox-rpi/redox-rpi/issues/500) 2 GB card | should | release | #24 | rc3 card (1 301 282 816 bytes) | verified |

**Not in the v0.6.0 scope:** the Pi 400 set, REQ-BOARD-4
([#350](https://github.com/redox-rpi/redox-rpi/issues/350)), REQ-USB-5,
REQ-GFX-3, REQ-AUDIO-4, REQ-NET-8, REQ-SD-4, REQ-NET-9 (#537–#542): all
proposed, no Pi 400 image in v0.6.0.

## Summary

- **45 requirements** in the release's scope (v0.5.0's 43 with REQ-AUDIO-3,
  REQ-NET-7 new, and REQ-NET-1, REQ-NET-2, REQ-FW-3, REQ-GFX-2, REQ-TCC-1
  reworded).
- **Fails:** REQ-NET-4 criterion 2 (rc1, memory grows over 1000
  connections; not run again).
- **Out of date:** REQ-BOOT-1 criteria 2 and 3 (root from RAM on the direct
  card, #324).
- **Better than v0.5.0:** REQ-FW-3 power off (2, 3), REQ-NET-1 criteria 5
  and 6, REQ-BOARD-1, REQ-NET-2 and REQ-CON-1 criterion 3 on the candidate
  (Zero 2 W), REQ-NET-7 on the Pi, REQ-TCC-1 criterion 5.
- **Known deviation:** REQ-AUDIO-1 and REQ-AUDIO-2: the jack is silent
  with `hdmi-audio`, which rc3 adds. The maintainer accepted it for v0.6.0
  on 2026-11-22; a choice between jack and HDMI follows in #617 (v0.7.0). HDMI sound itself has no requirement in
  redox-rpi yet (#423).

### Must criteria without a test on the candidate

For the checklist's step "Untested `must` criteria":

| Requirement | Criterion | State |
|---|---|---|
| REQ-BOOT-1 | 2, 3 | out of date: root from RAM by design (#324) |
| REQ-NET-1 | 2, 3 | not tested (new wording; PR #614 for v0.7.0) |
| REQ-NET-1 | 6 | rc1 only (450 B, `+xc80e`) |
| REQ-NET-4 | 2 | **fails** on rc1 (450 C, `+xc80e`) |
| REQ-NET-7 | 4 | not tested (name server not answering); requirement still proposed |
| REQ-REL-2 | 3 | `/.uuid` not checked |
| REQ-REL-3 | 3 | QEMU boot test green on the rc1 commit only |
| REQ-REL-4 | 1, 2 | at release |
| REQ-FW-3, REQ-REL-1 | 2, 3 / 2 | rc2 only (571, `+xcbcb`); rc3 changed only audio |

## Findings for the other roles

- **Maintainer:** accept or test the must list above. (The silent jack
  is accepted, #617.) HDMI sound (#423) needs a requirement if it stays in the release.
- **V&V:** HDMI mode on the 3B+ came up with a monitor (575), so 573's
  `hdmi_mode no` was the setup. Overscan border on the 3B+: #347/#348.
- **Network (#88):** REQ-NET-4 criterion 2 fails; memory grows outside
  any process (kernel?).
- **Requirements (me):** reword REQ-BOOT-1 criteria 2 and 3 for the direct
  card (#324); a requirement for HDMI sound (#423).
