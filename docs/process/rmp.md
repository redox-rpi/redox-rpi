# Requirements management plan (RMP)

How redox-rpi finds, writes, reviews, changes and traces its requirements.
Part of the [project management plan](pmp.md#requirements-management);
work package 1.3 of the [WBS](wbs.md). Owned by the Requirements agent;
changed by pull request, approved by the maintainer.

Status: draft, 2026-11-08; updated 2026-11-10 for RFC 0002 (accepted):
requirements are Feature issues in `redox-rpi`. Open points are listed at
the end.

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

Since [RFC 0002](../rfcs/0002-project-structure.md#4-issues-types-labels-and-the-project)
(accepted 2026-11-09) **each requirement is one GitHub issue of type
Feature in `redox-rpi`**. The Feature issue is the record; tasks and bugs
that implement it hang below it as sub-issues, in the repository of their
code.

### Issue format

- **Type:** Feature. **Repository:** `redox-rpi`.
- **Title:** `REQ-<AREA>-<n> <name>`, for example
  `REQ-NET-1 DHCP on the onboard Ethernet`.
- **Labels:** the `topic:<name>` of the area (the shared label list of RFC
  0002; no extra labels for priority or level).
- **Milestone:** the release in which the requirement must first hold.
- **Project fields:** Release, Type, Topic, Owner, as for every issue.
- **Body**, in this order:

```markdown
**Statement:** The system shall …

| Field | Value |
|---|---|
| Status | proposed / approved / retired |
| Kind | functional / quality / constraint |
| Priority | must / should / may |
| Level | release / beta / experiment / test image |
| PBS | 2.3.4, 2.4.2 |
| Source | maintainer decision 2026-11-08 / issue / release |
| Since | v0.1.0 |
| Verification | test, automated: checklist test 1, pi-tests row 285 |

**Rationale:** …

### Acceptance criteria

- [ ] 1. … (pi-tests row N on <build ID>)
- [ ] 2. …

### Trace

Implemented by: the sub-issues and PRs. Verified by: pi-tests rows and
reports.
```

V&V ticks an acceptance box with the row and build ID that passed it, for
the current release candidate; the boxes are cleared when the next
candidate is tested (the per-release trace keeps the old results).

### Existing Feature issues

Where a Feature issue for the same capability already exists (for example
#250 PWM audio, #45 SSH, #36 keyboard layouts), the requirement goes into
that issue: its title gets the ID and its body the requirement block above
the existing text. One requirement per Feature issue; a Feature issue that
covers several capabilities keeps one requirement, and the others become
new Feature issues linked to it.

### What stays in files

- `docs/requirements/README.md`: the list of areas with a search link for
  each (`is:issue type:Feature "REQ-NET-" in:title`).
- `docs/requirements/trace-<release>.md`: the trace of each release,
  written when its candidate is tested; this is the release's baseline.
- Until the move, the first set lives in files (draft PR #270). The move
  creates or updates the issues with `scripts/req-issues.py` and then
  deletes the area files.

## Lifecycle

| Status | Meaning | Who sets it |
|---|---|---|
| proposed | Feature issue with `Status: proposed`, no milestone | Requirements agent |
| approved | the maintainer approves in a comment ("approved"); the Requirements agent then sets `Status: approved` and the milestone | maintainer |
| retired | issue closed as "not planned", `Status: retired` with the reason and date | maintainer, on a proposal |

The **issue state** says whether the requirement is met:

| Issue state | Meaning |
|---|---|
| open | approved (or proposed), not yet verified in a release |
| closed as completed | verified in the release of its milestone; the requirement stays in force |
| closed as not planned | retired |

A later failure of a met requirement is a **Bug** that names the ID
(`breaks REQ-NET-1`); the Feature issue stays closed, and the release
trace shows `fails`. Whether a requirement is **verified** for a given
build is kept in the per-release trace.

## Review and approval

| Who | Checks |
|---|---|
| Requirements agent | writes the requirement, follows the writing rules, links source and PBS |
| Topic owner of the area | the statement is right and feasible, the PBS and issues are right |
| V&V | each criterion is testable, and names the test or says **gap** |
| Quality | format and trace complete (may audit, does not block) |
| Project lead | priority fits the milestone plan |
| Maintainer | approves: requirements are decisions, so only the maintainer approves them (a comment on the Feature issue; for the first set, merging #270); the project lead never approves them for the maintainer |

The Requirements agent asks topic owners through the project lead. A
topic owner who finds a requirement wrong or missing tells the
Requirements agent the same way, or opens an issue for it.

## Changes

1. Anyone may propose a change: an issue, or a note to the Requirements
   agent through the project lead.
2. The Requirements agent edits the Feature issue (GitHub keeps the edit
   history) and adds a comment with the reason and an **impact note**:
   which issues, tests, checklists and release notes it touches. The
   status goes back to `proposed` until approved.
3. Reviewers comment on the issue; the maintainer approves in a comment.
4. After approval the Requirements agent updates the trace and tells the
   owners named in the impact note (through the project lead).

**Baseline.** When a release candidate is built, the approved requirements
of that release are its baseline: the release checklist lists their IDs and
the trace is written to `docs/requirements/trace-<release>.md`. A change after that goes into the
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
3. Every Feature issue in a milestone is a requirement (has a `REQ-` ID) or says why not (for example an idea or an effort analysis).
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

1. **Approval of the first set:** the maintainer approves the v0.4.0 set
   in draft PR #270; then `scripts/req-issues.py` creates or updates the
   Feature issues and the area files go away.
2. **Check script** for the trace checks (reads the Feature issues and the
   pi-tests reports): after the move.
3. **Areas `PKG`, `SEC`, `DOC`** from RFC 0003: added to the area table
   when RFC 0003 is accepted.
