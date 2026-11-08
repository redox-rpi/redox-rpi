# Requirements

What redox-rpi must do, one requirement per entry, each with an ID, a test
that verifies it and the issues that implement it. Owner: the Requirements
agent; the rules are in the [RMP](../process/rmp.md). The maintainer decides on every
requirement.

Status: draft, 2026-11-08. The first set describes what v0.4.0 meets or
claims (rc1, plus the optional desktop planned for rc2).

**This is a collection, not the final format.** The maintainer proposed
managing requirements as GitHub issues of type **Feature** (the org has
Task, Bug and Feature). Where those issues live depends on the revised RFC
0002 (repo split). Until then the files below collect the content so that
each requirement moves into one Feature issue unchanged; see
[As Feature issues](#as-feature-issues) and
[RMP, Storage](../process/rmp.md#storage).

## As Feature issues

Each `### REQ-…` entry is written to become one issue:

| Entry | Feature issue |
|---|---|
| ID and name | title `REQ-NET-1 DHCP on the onboard Ethernet` (the ID stays the stable reference in rows, issues and release notes; code comments name the issue) |
| Statement, rationale | first lines of the body |
| Acceptance criteria | a checklist in the body (`- [ ] …`); V&V ticks a box with the row that passed it |
| Priority, kind, level, PBS, since | labels (`prio:must`, `level:beta`) or project fields, as RFC 0002 decides |
| Status | open = in force, closed as "not planned" = retired; proposal = the issue is still in review |
| Area | the repo the requirement concerns (after the split) plus the `topic:<name>` label |
| Implemented by | the implementing Tasks and Bugs as sub-issues or "related" links |
| Verification | links to the Pi rows and reports, one comment per release with the state from [trace.md](trace.md) |

The trace table then becomes a query or project view over these issues;
until then [trace.md](trace.md) holds it. Moving them is one script run
(`gh issue create` per entry), done once the maintainer has approved the
content and RFC 0002 says which repo gets which area.

## Files

| File | Area | Topic page |
|---|---|---|
| [boot.md](boot.md) | `BOOT` boot from SD card and network, boot time, clean boot | [boot](../topics/boot.md) |
| [console.md](console.md) | `CON` HDMI and serial console | [boot](../topics/boot.md), [stability](../topics/stability.md) |
| [usb.md](usb.md) | `USB` keyboard, mouse, hotplug | [usb](../topics/usb.md) |
| [network.md](network.md) | `NET` Ethernet, DHCP, TCP, multicast | [network](../topics/network.md) |
| [time.md](time.md) | `TIME` clock and NTP | [time](../topics/time.md) |
| [sdcard.md](sdcard.md) | `SD` SD card and RedoxFS | [sdcard](../topics/sdcard.md) |
| [gpio.md](gpio.md) | `GPIO` GPIO, I2C, SPI | [gpio](../topics/gpio.md) |
| [firmware.md](firmware.md) | `FW` CPU clock, firmware interface, reboot | [firmware](../topics/firmware.md) |
| [audio.md](audio.md) | `AUDIO` headphone jack (beta) | [audio](../topics/audio.md) |
| [graphics.md](graphics.md) | `GFX` optional desktop (beta, rc2) | [graphics](../topics/graphics.md) |
| [tcc.md](tcc.md) | `TCC` native C compiler (experiment) | [tcc](../topics/tcc.md) |
| [stability.md](stability.md) | `STAB` program starts, kernel, relibc | [stability](../topics/stability.md), [relibc](../topics/relibc.md) |
| [boards.md](boards.md) | `BOARD` supported boards | [boards](../topics/boards.md), [zero2w](../topics/zero2w.md) |
| [release.md](release.md) | `REL` release image contents and publishing | [ci](../topics/ci.md), [release-process](../release-process.md) |
| [trace.md](trace.md) | trace table: requirement → issue → test → state | |

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

"Row N" is the number of a test in [test-plan.md](../test-plan.md); its
report is the heading `#N` in [test-reports.md](../test-reports.md).

## Trace

The chain from the [PMP](../process/pmp.md#traceability): requirement →
issue → branch and PR → code → Pi row → release notes.

- An issue that implements a requirement names its ID in the body
  (`Implements REQ-SD-2`).
- A Pi row or checklist test names the IDs it verifies (`verifies
  REQ-SD-1, REQ-SD-2`). V&V adds them; for the first set the trace table
  names the existing rows.
- [trace.md](trace.md) lists every requirement with its issues, tests and
  state for the current release. Quality turns its gaps into KPIs.

States in the trace table:

| State | Meaning |
|---|---|
| verified | a test passed on a reproducible build of the release or candidate |
| earlier | passed on an earlier build (a dev build or an earlier release), not yet on this candidate |
| queued | the test is in the queue for this candidate |
| fails | the test ran and failed; a known issue names it |
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
