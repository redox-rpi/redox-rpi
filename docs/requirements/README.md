# Requirements

What redox-rpi must do, one requirement per entry, each with an ID, a test
that verifies it and the issues that implement it. Owner: the Requirements
agent; the rules are in the [RMP](../process/rmp.md). The maintainer decides on every
requirement.

Status: draft, 2026-11-16. Since 2026-11-16 every requirement is a Feature
issue; this page keeps the list of areas, the search links and the
per-release traces ([RMP, Storage](../process/rmp.md#storage)).

## Feature issues

Since 2026-11-16 every requirement is a Feature issue, and the issue is the
record ([RMP, Storage](../process/rmp.md#storage)). Search:
[all requirements](https://github.com/search?q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-%22+in%3Atitle&type=issues).
Created from the former area files (git history) with a one-off script against
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

## Areas

| Area | What | Feature issues | Topic page |
|---|---|---|---|
| `BOOT` | boot from SD card and network, boot time, clean boot | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-BOOT-%22+in%3Atitle) | [boot](../topics/boot.md) |
| `CON` | HDMI and serial console | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-CON-%22+in%3Atitle) | [boot](../topics/boot.md) |
| `USB` | keyboard, mouse, hotplug | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-USB-%22+in%3Atitle) | [usb](../topics/usb.md) |
| `NET` | Ethernet, DHCP, TCP, multicast, DNS | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-NET-%22+in%3Atitle) | [network](../topics/network.md) |
| `TIME` | clock and NTP | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-TIME-%22+in%3Atitle) | [time](../topics/time.md) |
| `SD` | SD card and RedoxFS | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-SD-%22+in%3Atitle) | [sdcard](../topics/sdcard.md) |
| `GPIO` | GPIO, I2C, SPI | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-GPIO-%22+in%3Atitle) | [gpio](../topics/gpio.md) |
| `FW` | CPU clock, firmware interface, reboot | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-FW-%22+in%3Atitle) | [firmware](../topics/firmware.md) |
| `AUDIO` | headphone jack, startup sound, volume (beta) | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-AUDIO-%22+in%3Atitle) | [audio](../topics/audio.md) |
| `GFX` | optional desktop and its apps (beta) | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-GFX-%22+in%3Atitle) | [graphics](../topics/graphics.md) |
| `TCC` | native C compiler (beta) | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-TCC-%22+in%3Atitle) | [tcc](../topics/tcc.md) |
| `STAB` | program starts, kernel, relibc | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-STAB-%22+in%3Atitle) | [stability](../topics/stability.md) |
| `BOARD` | supported boards | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-BOARD-%22+in%3Atitle) | [boards](../topics/boards.md) |
| `REL` | release image contents and publishing | [issues](https://github.com/search?type=issues&q=org%3Aredox-rpi+is%3Aissue+type%3AFeature+%22REQ-REL-%22+in%3Atitle) | [ci](../topics/ci.md) |
| `VPU` | open VPU firmware (rpi-open-firmware) | [issues](https://github.com/redox-rpi/rpi-open-firmware/issues?q=REQ-VPU+in%3Atitle) | [open-firmware](../topics/open-firmware.md) |

Areas follow the topic pages. A new topic that changes what the system does
gets its own area code.

## Files

| File | What |
|---|---|
| [trace-v0.4.0.md](trace-v0.4.0.md) | trace for v0.4.0 |
| [trace-v0.5.0.md](trace-v0.5.0.md) | trace for v0.5.0, and the requirement updates it shows for v0.6.0 (draft) |
| [v1-criteria.md](v1-criteria.md) | RFC 0003 v1.0 criteria: Feature issues, trace, gaps (draft, parked) |
| [v1-work-packages.md](v1-work-packages.md) | proposed work packages for the 8 v1.0 criteria with no implementing issue (draft, parked) |
| [v1-test-drafts.md](v1-test-drafts.md) | test-row drafts for V&V (draft, parked) |

The requirement texts until 2026-11-16 are in the git history of this
folder (one file per area).

## Trace

The chain from the [PMP](../process/pmp.md#traceability): requirement →
issue → branch and PR → code → Pi row → release notes.

- An issue that implements a requirement names its ID in the body
  (`Implements REQ-SD-2`).
- A Pi row or checklist test names the IDs it verifies (`verifies
  REQ-SD-1, REQ-SD-2`). V&V adds them; for the first set the trace table
  names the existing rows.
- `trace-<release>.md` lists every requirement with its issues, tests and
  state for that release. Quality turns its gaps into KPIs.

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
([REQ-REL-1](https://github.com/redox-rpi/redox-rpi/issues/496)).
The automated rows therefore test the netboot image built from the same
commit and options plus `devshd`; only the visit tests the SD release image
itself.

## Changing requirements

In the Feature issue, with a comment that gives the reason and an impact
note; the maintainer approves in a comment: see [RMP, Changes](../process/rmp.md#changes).
