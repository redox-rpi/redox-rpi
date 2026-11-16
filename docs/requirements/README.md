# Requirements

What redox-rpi must do, one requirement per entry, each with an ID, a test
that verifies it and the issues that implement it. Owner: the Requirements
agent; the rules are in the [RMP](../process/rmp.md). The maintainer decides on every
requirement.

Status: draft, 2026-11-10. The first set describes what v0.4.0 meets,
verified in [trace-v0.4.0.md](trace-v0.4.0.md).

**These files are the input for the move to Feature issues.** Since RFC
0002 (accepted) each requirement is one Feature issue in `redox-rpi`; the
issue format is in [RMP, Storage](../process/rmp.md#storage). When the
maintainer approves this set (by merging the PR that brings it), the
Requirements agent runs `scripts/req-issues.py`, which creates or updates
one issue per `### REQ-…` entry. Then the area files are deleted, and this
page keeps the list of areas, the search links and the per-release traces.

## Feature issues

Since 2026-11-16 every requirement is a Feature issue, and the issue is the
record ([RMP, Storage](../process/rmp.md#storage)). Search:
[all requirements](https://github.com/search?q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-%22+in%3Atitle&type=issues).
Created from these files with `scripts/req-issues.py` against
[trace-v0.5.0.md](trace-v0.5.0.md): requirements verified in full on
v0.5.0 are closed as completed with milestone v0.5.0; the four new or
changed for v0.6.0 (REQ-AUDIO-3, REQ-NET-7, REQ-GFX-2, REQ-TCC-1) are
open with Status proposed and no milestone. Firmware requirements
(`REQ-VPU-…`) live in
[rpi-open-firmware](https://github.com/redox-rpi/rpi-open-firmware/issues?q=REQ-VPU+in%3Atitle).

| Requirement | Issue |
|---|---|
| REQ-AUDIO-1 | [#250](https://github.com/redox-rpi/redox-rpi/issues/250) |
| REQ-AUDIO-2 | [#472](https://github.com/redox-rpi/redox-rpi/issues/472) |
| REQ-AUDIO-3 | [#473](https://github.com/redox-rpi/redox-rpi/issues/473) |
| REQ-BOARD-1 | [#474](https://github.com/redox-rpi/redox-rpi/issues/474) |
| REQ-BOOT-1 | [#475](https://github.com/redox-rpi/redox-rpi/issues/475) |
| REQ-BOOT-2 | [#476](https://github.com/redox-rpi/redox-rpi/issues/476) |
| REQ-BOOT-3 | [#477](https://github.com/redox-rpi/redox-rpi/issues/477) |
| REQ-BOOT-4 | [#478](https://github.com/redox-rpi/redox-rpi/issues/478) |
| REQ-BOOT-5 | [#479](https://github.com/redox-rpi/redox-rpi/issues/479) |
| REQ-CON-1 | [#480](https://github.com/redox-rpi/redox-rpi/issues/480) |
| REQ-CON-2 | [#481](https://github.com/redox-rpi/redox-rpi/issues/481) |
| REQ-FW-1 | [#482](https://github.com/redox-rpi/redox-rpi/issues/482) |
| REQ-FW-2 | [#483](https://github.com/redox-rpi/redox-rpi/issues/483) |
| REQ-FW-3 | [#484](https://github.com/redox-rpi/redox-rpi/issues/484) |
| REQ-FW-4 | [#485](https://github.com/redox-rpi/redox-rpi/issues/485) |
| REQ-GFX-1 | [#489](https://github.com/redox-rpi/redox-rpi/issues/489) |
| REQ-GFX-2 | [#490](https://github.com/redox-rpi/redox-rpi/issues/490) |
| REQ-GPIO-1 | [#486](https://github.com/redox-rpi/redox-rpi/issues/486) |
| REQ-GPIO-2 | [#487](https://github.com/redox-rpi/redox-rpi/issues/487) |
| REQ-GPIO-3 | [#488](https://github.com/redox-rpi/redox-rpi/issues/488) |
| REQ-NET-1 | [#491](https://github.com/redox-rpi/redox-rpi/issues/491) |
| REQ-NET-2 | [#492](https://github.com/redox-rpi/redox-rpi/issues/492) |
| REQ-NET-3 | [#493](https://github.com/redox-rpi/redox-rpi/issues/493) |
| REQ-NET-4 | [#494](https://github.com/redox-rpi/redox-rpi/issues/494) |
| REQ-NET-5 | [#495](https://github.com/redox-rpi/redox-rpi/issues/495) |
| REQ-NET-7 | [#471](https://github.com/redox-rpi/redox-rpi/issues/471) |
| REQ-REL-1 | [#496](https://github.com/redox-rpi/redox-rpi/issues/496) |
| REQ-REL-2 | [#497](https://github.com/redox-rpi/redox-rpi/issues/497) |
| REQ-REL-3 | [#498](https://github.com/redox-rpi/redox-rpi/issues/498) |
| REQ-REL-4 | [#499](https://github.com/redox-rpi/redox-rpi/issues/499) |
| REQ-REL-5 | [#500](https://github.com/redox-rpi/redox-rpi/issues/500) |
| REQ-SD-1 | [#501](https://github.com/redox-rpi/redox-rpi/issues/501) |
| REQ-SD-2 | [#502](https://github.com/redox-rpi/redox-rpi/issues/502) |
| REQ-SD-3 | [#503](https://github.com/redox-rpi/redox-rpi/issues/503) |
| REQ-STAB-1 | [#504](https://github.com/redox-rpi/redox-rpi/issues/504) |
| REQ-STAB-2 | [#505](https://github.com/redox-rpi/redox-rpi/issues/505) |
| REQ-STAB-3 | [#506](https://github.com/redox-rpi/redox-rpi/issues/506) |
| REQ-TCC-1 | [#507](https://github.com/redox-rpi/redox-rpi/issues/507) |
| REQ-TIME-1 | [#508](https://github.com/redox-rpi/redox-rpi/issues/508) |
| REQ-TIME-2 | [#509](https://github.com/redox-rpi/redox-rpi/issues/509) |
| REQ-TIME-3 | [#51](https://github.com/redox-rpi/redox-rpi/issues/51) |
| REQ-USB-1 | [#510](https://github.com/redox-rpi/redox-rpi/issues/510) |
| REQ-USB-2 | [#511](https://github.com/redox-rpi/redox-rpi/issues/511) |
| REQ-USB-3 | [#512](https://github.com/redox-rpi/redox-rpi/issues/512) |
| REQ-USB-4 | [#513](https://github.com/redox-rpi/redox-rpi/issues/513) |

## Move to Feature issues (done)

| Entry | Feature issue |
|---|---|
| ID and name | title, e.g. `REQ-NET-1 DHCP on the onboard Ethernet` |
| Statement, field table, rationale | body, in the RMP's order, with a `Source` row |
| Acceptance criteria | a checklist; boxes ticked for the criteria verified on v0.4.0, with the row |
| Area | the `topic:<name>` label of the area |
| State | verified on v0.4.0 in all its criteria: **closed as completed**, milestone v0.4.0; otherwise **open**, milestone v0.4.0 kept for what was met and the open criteria named in a comment |
| Implemented by | sub-issues where an issue still exists; closed issues and PRs as links |

Requirements that go into an **existing Feature issue** instead of a new
one (its title gets the ID, its body the requirement block on top):

| Requirement | Existing Feature issue |
|---|---|
| REQ-AUDIO-1 | #250 PWM audio driver for the headphone jack |
| REQ-TIME-3 | #51 relibc: clock_settime and settimeofday; timesync -s |

The other 41 become new issues. #7 (power off) and #36 (keyboard layouts)
belong to the v1.0 criteria of [RFC 0003](../rfcs/0003-v1-criteria.md)
(REQ-FW-5, REQ-CON-3) and get their requirement when that RFC is decided.

## Files

| File | Area | Topic page |
|---|---|---|
| [boot.md](boot.md) | `BOOT` boot from SD card and network, boot time, clean boot | [boot](../topics/boot.md) |
| [console.md](console.md) | `CON` HDMI and serial console | [boot](../topics/boot.md), [stability](../topics/stability.md) |
| [usb.md](usb.md) | `USB` keyboard, mouse, hotplug | [usb](../topics/usb.md) |
| [network.md](network.md) | `NET` Ethernet, DHCP, TCP, multicast, DNS | [network](../topics/network.md) |
| [time.md](time.md) | `TIME` clock and NTP | [time](../topics/time.md) |
| [sdcard.md](sdcard.md) | `SD` SD card and RedoxFS | [sdcard](../topics/sdcard.md) |
| [gpio.md](gpio.md) | `GPIO` GPIO, I2C, SPI | [gpio](../topics/gpio.md) |
| [firmware.md](firmware.md) | `FW` CPU clock, firmware interface, reboot | [firmware](../topics/firmware.md) |
| [audio.md](audio.md) | `AUDIO` headphone jack, startup sound, volume (beta) | [audio](../topics/audio.md) |
| [graphics.md](graphics.md) | `GFX` optional desktop and its apps (beta) | [graphics](../topics/graphics.md) |
| [tcc.md](tcc.md) | `TCC` native C compiler (beta) | [tcc](../topics/tcc.md) |
| [stability.md](stability.md) | `STAB` program starts, kernel, relibc | [stability](../topics/stability.md), [relibc](../topics/relibc.md) |
| [boards.md](boards.md) | `BOARD` supported boards | [boards](../topics/boards.md), [zero2w](../topics/zero2w.md) |
| [release.md](release.md) | `REL` release image contents and publishing | [ci](../topics/ci.md), [release-process](../release-process.md) |
| [trace-v0.4.0.md](trace-v0.4.0.md) | trace table: requirement → issue → test → state | |
| [trace-v0.5.0.md](trace-v0.5.0.md) | trace for v0.5.0, and the requirement updates it shows for v0.6.0 (draft) | |
| [v1-criteria.md](v1-criteria.md) | RFC 0003 v1.0 criteria: Feature issues, trace, gaps (draft, parked) | |
| [v1-work-packages.md](v1-work-packages.md) | proposed work packages for the 8 criteria with no implementing issue (draft, parked) | |
| [v1-test-drafts.md](v1-test-drafts.md) | test-row drafts for V&V, 20 criteria with no candidate test (draft, parked) | |

Areas follow the topic pages. A new topic that changes what the system does
gets its own area code and file.

## Format

Each requirement is a level-3 heading with its ID and a short name, then a
small table and the acceptance criteria:

```markdown
### REQ-NET-1 DHCP on the onboard Ethernet

The system shall get an IPv4 address by DHCP on the Pi 3B+'s onboard
Ethernet (LAN7800) at boot.

| Field | Value |
|---|---|
| Rationale | Remote use, NTP and network boot need the network. |
| Priority | must |
| Kind | functional |
| Level | release |
| PBS | 2.3.4, 2.4.2 |
| Status | proposed |
| Since | v0.1.0 |
| Implemented by | #… |
| Verification | test, automated: checklist v0.4.0 test 1, row 285 |

Acceptance criteria:

1. Within 60 s after power-on, `/scheme/netcfg/ifaces/eth0/addr/list`
   shows the address the DHCP server gave.
2. …
```

The fields, the writing rules, the areas with their PBS and WBS parts,
review, changes and verification methods are defined in the
[requirements management plan](../process/rmp.md) (RMP). This page only
lists the files and the conventions of this first set.

"Row N" is the number of a test row in the
[pi-tests queue](https://github.com/redox-rpi/pi-tests/blob/main/queue.md);
its report is `reports/N.md` in
[redox-rpi/pi-tests](https://github.com/redox-rpi/pi-tests/tree/main/reports)
(rows before v0.4.0 development, such as 5, 35, 67, 83 and 161, are in
`reports/archive-before-v0.4.0-dev.md`). Release evidence is in pi-tests
`releases/`.

## Trace

The chain from the [PMP](../process/pmp.md#traceability): requirement →
issue → branch and PR → code → Pi row → release notes.

- An issue that implements a requirement names its ID in the body
  (`Implements REQ-SD-2`).
- A Pi row or checklist test names the IDs it verifies (`verifies
  REQ-SD-1, REQ-SD-2`). V&V adds them; for the first set the trace table
  names the existing rows.
- [trace-v0.4.0.md](trace-v0.4.0.md) lists every requirement with its issues, tests and
  state for the current release. Quality turns its gaps into KPIs.

States in the trace table:

| State | Meaning |
|---|---|
| verified | a test passed on a reproducible build of the release or candidate |
| earlier | passed on an earlier build (a dev build or an earlier release), not yet on this candidate |
| queued | the test is in the queue for this candidate |
| fails | the test ran and failed; a known issue names it |
| planned | a method is agreed with V&V or Release & CI, but has not run yet |
| gap | no test checks it |

**What the automated rows verify.** `scripts/pi-test.py` logs in through
`devshd`, which the release image does not contain
([REQ-REL-1](release.md#req-rel-1-no-remote-shell-in-the-release-image)).
The automated rows therefore test the netboot image built from the same
commit and options plus `devshd`; only the visit tests the SD release image
itself.

## Changing requirements

By pull request, approved and merged by the maintainer, with an impact
note: see [RMP, Changes](../process/rmp.md#changes).
