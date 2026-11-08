# Requirements

What redox-rpi must do, one requirement per entry, each with an ID, a test
that verifies it and the issues that implement it. Owner: the Requirements
agent ([PMP](../process/pmp.md)). The maintainer decides on every
requirement.

Status: draft, 2026-11-08. The first set describes what v0.4.0 meets or
claims (rc1, plus the optional desktop planned for rc2).

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
| Level | release |
| Status | proposed |
| Since | v0.1.0 |
| Issues | #… |
| Verified by | checklist v0.4.0 test 1, row 285 |

Acceptance criteria:

1. Within 60 s after power-on, `/scheme/netcfg/ifaces/eth0/addr/list`
   shows the address the DHCP server gave.
2. …
```

| Field | Meaning |
|---|---|
| ID | `REQ-<AREA>-<n>`. Numbers count up per area and are never reused, also when a requirement is retired. |
| Statement | One sentence, "The system shall …", what and not how. |
| Rationale | Why it is needed, in one or two sentences. |
| Priority | `must` (a release fails without it), `should` (a failure is a known issue in the release notes), `may` (nice to have). |
| Level | `release` (in the release image), `beta` (in the release image, limits in the notes), `experiment` (on `main` but only with `--experiment`), `test image` (netboot and test images only). |
| Status | `proposed` (in a PR), `approved` (merged by the maintainer), `retired` (kept with a line why). |
| Since | the first release that met it. |
| Issues | the issues and PRs that implement it. |
| Verified by | the checklist test (`checklist v0.4.0 test 6`) and the Pi row or report (`row 285`) that shows it, or **gap** when no test exists. |
| Acceptance criteria | numbered, each one a check V&V can run and pass or fail, with the numbers that decide it. |

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

- A new or changed requirement comes as a pull request against
  `docs/requirements/`. Requirements are decisions: the maintainer reviews
  and merges them, the project lead does not merge them as docs-only PRs.
- Topic owners, V&V or Quality who find a requirement unclear, wrong or
  missing ask the Requirements agent through the project lead, or open an
  issue with the label `topic:requirements`.
- A requirement is retired, not deleted: its status becomes `retired` with
  the reason and the date.
- Numbers in acceptance criteria (times, rates, tolerances) are part of the
  decision. A proposed number is marked "(proposed)" until the maintainer
  confirms it.
- When a release adds a feature, its requirement is written before the
  candidate is built, so the checklist can name it.
