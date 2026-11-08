# Requirements management plan (RMP)

How redox-rpi finds, writes, reviews, changes and traces its requirements.
Part of the [project management plan](pmp.md#requirements-management);
work package 1.3 of the [WBS](wbs.md). Owned by the Requirements agent;
changed by pull request, approved by the maintainer.

Status: draft, 2026-11-08. Open points are listed at the end; the main one
is where requirements are stored (files or GitHub Feature issues), which
waits for the revised RFC 0002.

## Purpose

A requirement says **what** the system must do and how we know it does it.
It is the start of the trace chain in the PMP:

    source → requirement (REQ-…) → PBS part → issue → PR → code → test (Pi row) → release notes

With requirements, a release can say what it promises, V&V knows what to
test, and Quality can count what is promised but not tested.

## Scope

Requirements cover the products of the [PBS](pbs.md): the SD card image
(PBS 2) on the supported boards (PBS 1), and the parts of the build and
test system (PBS 3) and the releases (PBS 4) that a user or a release
depends on, such as reproducible builds and checksums. Internal
conventions of the team (coding style, test format) are standards, not
requirements; they live in `docs/process/`.

## Sources

| Source | Example | How it becomes a requirement |
|---|---|---|
| Maintainer goals and decisions | "optional desktop from rc2, as a beta" | The project lead passes it on; the Requirements agent writes it and cites the decision (date, issue or PR). |
| Roadmap and milestones | milestone v0.5.0: USB storage | Each feature in a milestone gets a requirement before its candidate is built. |
| Feature requests in issues | #36 German keyboard layout | When the maintainer accepts the request for a milestone. |
| Bugs | #217 boot without network | A bug shows a requirement that is missing or too weak; the fix gets one if none exists (here REQ-NET-1 criterion 3). |
| Users and the community | reports, Discussions | Through an issue; the maintainer decides. |
| What is already shipped | release notes, topic pages | A feature in a release is a promise: it gets a requirement, also after the fact (the v0.4.0 set). |
| Constraints | hardware limits, licences (vc4d is GPL-2.0-only), upstream Redox | Written as constraints with the same format, area of the part they bind. |
| V&V and Quality findings | a test that has no requirement, a KPI gap | Reported to the Requirements agent, who proposes a requirement or a change. |

Each requirement names its source in the rationale.

## Kinds and levels

- **Functional:** what the system does (`REQ-USB-1` keyboard input).
- **Quality:** how well: speed, reliability, resource use, security
  (`REQ-BOOT-4` boot time, `REQ-STAB-1` no failed starts). Always with a
  number.
- **Constraint:** a limit on how it is built or shipped (`REQ-REL-1` no
  unencrypted remote shell in the release image).

The **level** says in which images a requirement must hold:

| Level | Meaning |
|---|---|
| release | in the release image; a failure of a `must` stops the release |
| beta | in the release image with its limits in the release notes |
| experiment | on `main`, only in builds with `--experiment <name>` |
| test image | netboot and test images only (PBS 3) |

## Format

### ID

`REQ-<AREA>-<n>`, for example `REQ-NET-4`.

- `AREA` is a short code from the table below; `n` counts up per area.
- An ID is never reused or renumbered, also not when the requirement is
  retired or moved to another area.
- Code comments name the **issue**, not the requirement (coding
  standards); the issue names the requirement.

### Areas

Areas follow the topics, so each area has one topic owner who reviews it.
Each area lists the PBS parts its requirements usually belong to; every
requirement still names its own PBS ID.

| Area | Topic | PBS | WBS |
|---|---|---|---|
| `BOOT` boot chain, boot time | boot | 2.1, 2.4.6 | 2.1 |
| `CON` HDMI and serial console | boot | 2.3.1, 2.3.6, 2.4.6 | 2.1 |
| `STAB` program starts, kernel, C library | stability, relibc | 2.2.1, 2.4.1 | 2.2, 2.3 |
| `SD` SD card and RedoxFS | sdcard | 2.3.3, 2.4.3 | 2.4 |
| `USB` USB host, input, hotplug, storage | usb | 2.3.2 | 2.5 |
| `FW` firmware interface, CPU clock, reboot | firmware | 2.3.8, 2.2.1 | 2.6 |
| `NET` Ethernet, Wi-Fi, network stack, SSH | network | 2.3.4, 2.3.5, 2.4.2, 2.5.6 | 2.7 |
| `TIME` clock, NTP, time zones | time | 2.4.4, 2.3.11 | 2.7, 2.14 |
| `GPIO` GPIO, I2C, SPI | gpio | 2.3.9 | 2.8 |
| `GFX` display and desktop | graphics | 2.3.6, 2.3.7, 2.5.4 | 2.9 |
| `AUDIO` sound | audio | 2.3.10, 2.4.5 | 2.10 |
| `TCC` developer tools on the Pi | tcc | 2.5.3 | 2.11 |
| `ASST` assistant | assistant | 2.5.5 | 2.12 |
| `FWO` open-source VPU firmware | open-firmware | 2.1.2 | 2.13 |
| `BOARD` supported boards | boards, zero2w | 1 | 3 |
| `REL` release image, build, publishing | ci | 2.6, 3.1, 4 | 5 |

A new area is added by a change to this table.

### Fields

| Field | Content |
|---|---|
| ID and name | `REQ-NET-1 DHCP on the onboard Ethernet` |
| Statement | One sentence, "The system shall …": what, not how. |
| Rationale | Why, and the source (decision, issue, release). |
| Kind | functional, quality or constraint |
| Priority | `must` (a release fails without it), `should` (a failure is a known issue in the notes), `may` |
| Level | release, beta, experiment, test image |
| PBS | the PBS IDs of the parts that implement it (`2.3.4, 2.4.2`) |
| Status | proposed, approved, retired (see below) |
| Since | first release that met it |
| Implemented by | issues and PRs |
| Verification | method and test: see [Verification](#verification) |
| Acceptance criteria | numbered checks, each with a pass or fail result and the numbers that decide it |

### Writing rules

A requirement is:

1. **One thing.** Two behaviours are two requirements.
2. **Testable.** Every criterion can pass or fail on a named image; words
   like "fast", "stable" or "usually" need a number.
3. **What, not how.** Driver names belong in the rationale or the PBS
   field, not in the statement, so a new driver does not change it.
4. **Bound to a level and a board.** The Pi 3B+ is the default board;
   others are named.
5. **Honest about numbers.** A number nobody decided yet is marked
   "(proposed)" until the maintainer confirms it.

## Storage

The content above is fixed by this plan; where it is stored is not yet.

- **Today:** one Markdown file per area under `docs/requirements/`, one
  level-3 heading per requirement, plus `trace.md`.
- **Planned (proposal of the maintainer):** one GitHub issue of type
  **Feature** per requirement, in the repository of the part it concerns
  (after RFC 0002). Title = ID and name; body = statement, rationale and
  the acceptance criteria as a checklist; priority and level as labels or
  project fields; implementing Tasks and Bugs as sub-issues; verification
  links in the body. The files then shrink to an index and the trace becomes
  a project view.

The move is one script run per area, made once RFC 0002 says which
repository holds which area. Until then the files are the record.

## Lifecycle

| Status | Meaning | Who sets it |
|---|---|---|
| proposed | written, in a pull request (or an open Feature issue without the approval label) | Requirements agent |
| approved | merged by the maintainer (or approved on the issue) | maintainer |
| retired | no longer wanted; kept with the reason and date | maintainer, on a proposal |

Whether a requirement is **implemented** and **verified** is not a status
of the requirement: it depends on the build and is kept in the trace for
each release.

## Review and approval

| Who | Checks |
|---|---|
| Requirements agent | writes the requirement, follows the writing rules, links source and PBS |
| Topic owner of the area | the statement is right and feasible, the PBS and issues are right |
| V&V | each criterion is testable, and names the test or says **gap** |
| Quality | format and trace complete (may audit, does not block) |
| Project lead | priority fits the milestone plan |
| Maintainer | approves: requirements are decisions, so the maintainer merges them; they are never merged as docs-only changes |

The Requirements agent asks topic owners through the project lead. A
topic owner who finds a requirement wrong or missing tells the
Requirements agent the same way, or opens an issue for it.

## Changes

1. Anyone may propose a change: an issue, or a note to the Requirements
   agent through the project lead.
2. The Requirements agent writes the change as a pull request (later: an
   edit of the Feature issue) with the reason and an **impact note**:
   which issues, tests, checklists and release notes it touches.
3. Reviewers as above; the maintainer approves.
4. After approval the Requirements agent updates the trace and tells the
   owners named in the impact note (through the project lead).

**Baseline.** When a release candidate is built, the approved requirements
of that release are its baseline: the release checklist lists their IDs and
the trace is copied into the checklist. A change after that goes into the
next release unless the maintainer decides otherwise for this one.

**Retiring.** A requirement is retired, not deleted, so old rows, issues
and notes that name it still resolve.

## Verification

Every requirement names how it is verified:

| Method | When | Example |
|---|---|---|
| Test, automated | a Pi row or QEMU check without a person | pi-test.py starts (`REQ-STAB-1`) |
| Test, person | needs someone at the Pi (screen, keyboard, ears) | listening test (`REQ-AUDIO-2`) |
| Inspection | a look at the image, the build or a document | no `devshd` in the release file list (`REQ-REL-1`) |
| Analysis | measured or derived from logs and numbers | image size against a 2 GB card (`REQ-REL-5`) |

Rules, together with the test standards of V&V:

- A requirement is **verified for a release** only by a test on a
  reproducible build of that release or its candidate (no `--allow-dirty`).
- Automated Pi rows log in through `devshd`, so they run on the netboot
  image of the same commit, not on the SD release image. A requirement whose
  criteria depend on the SD image needs the person visit or an inspection.
- A criterion with no test is marked **gap**. Gaps are allowed in a draft;
  a `must` requirement of the release with a gap needs the maintainer's OK
  in the release checklist.

## Traceability

| Link | Kept where | Checked by |
|---|---|---|
| requirement → source | rationale | Requirements agent |
| requirement → PBS | PBS field | Requirements agent, Architect |
| requirement → issue | "Implemented by"; the issue names the ID (`Implements REQ-SD-2`) | Requirements agent, topic owners |
| issue → PR → code | PR names the issue; code comments name the issue | Quality (coding standards) |
| requirement → test | "Verification"; the Pi row or checklist test names the IDs it verifies | V&V |
| requirement → release | trace table per release, release checklist, release notes for beta limits and known issues | Requirements agent, Release and CI |

The **trace table** per release lists every requirement with its issues,
test and state (verified, earlier, queued, fails, gap). The Requirements
agent updates it when a candidate is tested and before the release.

**Checks** (by hand now, a script later):

1. Every requirement names a PBS ID and a source.
2. Every `must` and `should` of the release has a test or says **gap**.
3. Every Feature issue in a milestone links to a requirement.
4. Every requirement of the release has an implementing issue, or says it
   was there before the project (upstream Redox).
5. No test row names an ID that does not exist.

**KPIs** that Quality can take into its dashboard:

- requirements of the release without a test (by priority);
- `must` requirements without a passing test on the candidate;
- requirements verified only on an earlier build;
- issues in the milestone with no requirement.

## Roles

| Role | Does |
|---|---|
| Requirements agent | this plan, the requirements and their IDs, the trace table, the impact notes |
| Maintainer | sets goals, approves and retires requirements, decides proposed numbers |
| Project lead | passes on goals and questions, keeps priorities and milestones in line |
| Topic owners | review their area, link their issues, report missing requirements |
| V&V | review testability, name REQ IDs in rows and checklist tests |
| Quality | audit the trace, run the checks, show the KPIs |
| Release and CI | list the baseline in the release checklist, carry beta limits into the notes |

## Open points

1. **Storage:** files or Feature issues, and which repository per area:
   waits for the revised RFC 0002.
2. **Labels or fields** for priority and level (`prio:must`,
   `level:beta`), and an approval label: with the move to issues.
3. **Check script** for the trace checks: after the storage is decided.
4. The first set for v0.4.0 is draft PR #270; it follows this plan once
   the plan is approved.
