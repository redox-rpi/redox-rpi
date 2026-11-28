# Project management plan

How the redox-rpi project is organised: who does what, which documents
exist, and how work moves from an idea to a release. Its process rules are
owned by the Process session, the planning parts (sprints, work packages,
status) by the project lead. It is changed by pull request; the maintainer
approves it.

Status: in force; first draft 2026-11-08, last changed 2026-11-24.

## Goal

Redox OS on (almost) every Raspberry Pi from the Zero 2 W upwards, with
reproducible builds, tested on a real Pi, released every week as versioned
SD card images. The roadmap is in [ROADMAP.md](../../ROADMAP.md); its
themes are goals that take several sprints.

## Roles

The project lives in the GitHub organisation
[redox-rpi](https://github.com/redox-rpi) and is open to everyone: people
and AI agents are welcome to join, take a topic or a single issue, and grow
into an owner role. How to start is on the [join page](../join.md).

Today the team is one **maintainer** and a set of Claude agent sessions.
Each agent owns one area and keeps its knowledge in its own memory, so the
team stays stable; new agents are added only for expertise nobody has yet.
Every role below can also be held, or shared, by a contributor who joins.

| Role | Who | Owns |
|---|---|---|
| Maintainer | the maintainer (today one person) | goals and priorities, merging code PRs that change the image, releases and tags, RFC decisions, posts to the Redox community, tests that need a person at the Pi |
| Project lead | coordinator session | the planning parts of this plan (sprints, work packages, status), the coarse work packages and who owns them, overall status (`STATUS.md`), merge readiness (`merge-next`), docs-only merges, reporting to the maintainer |
| Process | Process session | the process rules: the process parts of this plan and its sub-plans (including the [firmware PMP](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/pmp.md)), [CONTRIBUTING.md](../../CONTRIBUTING.md), issue and PR templates, labels, rulesets, new-repository setup, the RFC procedure, process improvements from the sprint reviews. Process writes the rules; Quality checks them |
| Quality | Quality session | the [QMP](qmp.md): process compliance, bug analysis, KPIs and the KPI dashboard; merging build and test system PRs (see [Merge rights](#merge-rights), never its own). Quality does not test |
| Verification and validation (V&V) | Pi test agent, extended | test standards, test plan and reports, automated and special tests on QEMU and the real Pi, the regression set, preparing person tests |
| Requirements | Requirements session | the requirements, their IDs and their trace to issues, code and tests |
| Architect | Architect session | architecture, coding standards and coding rules, RFC drafts, cross-topic technical decisions |
| Release and CI | Release & CI session | builds, CI, release candidates and releases, the image server |
| Topic owners | one session per topic | their topic: design, code, bug fixes, topic page (see the table below) |
| Contributors | anyone, people or their agents | issues they take, following [CONTRIBUTING.md](../../CONTRIBUTING.md); a contributor can become a topic owner |

### Topic owners

| Topics (`topic:` label) | Owner session |
|---|---|
| graphics | Graphics (framebuffer driver `rpi-fbd`, display driver `vc4d`) |
| boot, sdcard, usb, firmware | Boot and console |
| network, time, gpio, smarthome | Network |
| stability (incl. chaos test) | Chaos test |
| relibc, ci | Release and CI |
| zero2w, verification | V&V |
| audio | Audio |
| tcc | TCC |
| assistant | Log triage (local language model) |
| quality | Quality |
| process | Process |
| open firmware (`topic:boot`, own repository) | Open firmware |
| VC4 tools (tinycc VC4 backend, resim, vc4-toolchain; own repositories) | VC4 Tools |
| boards (Pi 4, Pi 5, test lab), dma | no owner yet (project lead) |

A topic without an active owner falls back to the project lead, who hands
it on.

A topic owner breaks its coarse package into smaller work packages itself
(GitHub issues with the `topic:<name>` label), because it knows the area
best. It fixes the bugs in its area. Example: Graphics owns the new display
driver `vc4d` and splits it into packages, and keeps the framebuffer driver
`rpi-fbd`, including its bugs.

## Documents

| Document | Path | Owner | Status |
|---|---|---|---|
| Project management plan | `docs/process/pmp.md` | Process (process rules), project lead (planning) | in force |
| Firmware project management plan | [rpi-open-firmware `docs/pmp.md`](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/pmp.md) | Process (process rules), project lead (status) | draft |
| Product breakdown structure (PBS) | [`docs/process/pbs.md`](pbs.md) | project lead, Architect | draft |
| Work breakdown structure (WBS) | [`docs/process/wbs.md`](wbs.md) | project lead; process rules in it: Process | draft |
| Risks and opportunities | issues labelled [`risk`](https://github.com/redox-rpi/redox-rpi/issues?q=is%3Aopen+label%3Arisk) and [`opportunity`](https://github.com/redox-rpi/redox-rpi/issues?q=is%3Aopen+label%3Aopportunity) | project lead | live |
| Join page | [`docs/join.md`](../join.md) | project lead | draft |
| Quality management plan and KPI dashboard | `docs/process/qmp.md` | Quality | to write ([#648](https://github.com/redox-rpi/redox-rpi/issues/648)) |
| Requirements management plan (part of this plan) | [`docs/process/rmp.md`](rmp.md) | Requirements | draft |
| Requirements | Feature issues `REQ-…` ([RMP](rmp.md#storage)); traces in [`docs/requirements/`](../requirements/) | Requirements | live |
| Architecture | [`docs/architecture.md`](../architecture.md) | Architect | exists |
| Coding standards and rules | [`docs/process/coding-standards.md`](coding-standards.md) | Architect | draft, grows by section |
| Test standards | `docs/process/test-standards.md` | V&V | to write ([#264](https://github.com/redox-rpi/redox-rpi/issues/264)) |
| Test plan and reports | [redox-rpi/pi-tests](https://github.com/redox-rpi/pi-tests) (`queue.md`, `reports/`, `lab.md`, `releases/`); `docs/test-plan.md` and `docs/test-reports.md` point there | V&V | moved (RFC 0002 phase 2) |
| Release process and checklists | `docs/release-process.md`, `docs/releases/` | Release and CI | exist |
| Topic pages | `docs/topics/` | topic owners | exist |
| Contribution rules | `CONTRIBUTING.md` | Process | exists; links the documents above |
| Overall status | `STATUS.md` | project lead | to write ([#649](https://github.com/redox-rpi/redox-rpi/issues/649)) |

Every document marked "to write" links its issue. When a document is
written or its status changes, its author updates this table in the same
change.

### What the standards must cover

- **Coding standards and rules:** language and style per component (Rust,
  C, shell, Python), error handling, unsafe code, logging, patch format
  (small, close to upstream, one concern per patch), commit messages.
- **Documentation in code:** every bug fix and workaround names its issue at
  the place in the code, e.g. `// Workaround for #92: ...`, and in the patch
  header and commit message (`fixes #92`). A reader can go from the code to
  the bug and back.
- **Test standards:** test levels (QEMU, real Pi, person test), what a Pi
  row and a report must contain, pass criteria, how flaky results are
  handled, that results come only from reproducible builds (no
  `--allow-dirty`).
- **Requirements:** one ID per requirement (`REQ-<area>-<n>`), a short
  statement, acceptance criteria, the issue(s) that implement it and the
  test that verifies it.

## Requirements management

How requirements are found, written, reviewed, changed and traced is the
**requirements management plan** (`docs/process/rmp.md`), a part of this
plan written and kept by the Requirements agent. It covers at least: where
requirements come from (maintainer goals, roadmap, issues, users), the
format and IDs (`REQ-<area>-<n>`, see below), who reviews and approves
them, how a change to a requirement is handled, and how the trace to the
[PBS](pbs.md), issues and tests is kept and checked.

## Traceability

Requirement (`REQ-…`) → issue → branch and PR (references the issue) → code
(comment with the issue) → Pi row and report (names the requirement or
issue) → release notes. Quality checks that the chain is complete; the KPI
dashboard shows the gaps.

## Sprints and weekly releases

We work agile, without following Scrum strictly. The rhythm is one sprint
per week and one release per sprint:

- **What is ready goes in.** A change that is merged and Pi-tested by the
  cut-off is in the week's release. Everything else moves to the next
  sprint. A release never waits for a feature.
- **One week:** Monday to Sunday.

| When | What | Who |
|---|---|---|
| Monday, after the release | Sprint planning: the project lead proposes the sprint goal and the issues from the topic owners; the maintainer confirms or changes it | project lead, maintainer |
| Monday to Friday | Work, Pi tests, pull requests; `merge-next` kept current | owners, V&V, project lead |
| Friday evening | Cut-off: what the maintainer has merged by then is in | maintainer |
| Saturday | Release candidate built from `main`, candidate checks, Pi tests and the regression set | Release and CI, V&V |
| Sunday evening | Person test if one is due; the maintainer approves the tag on the exact commit; release notes and known issues. **Monday evening** is the fallback | maintainer, Release and CI |
| After the release | Sprint review: short report with what shipped, what moved, KPIs, and the risks and opportunities review; process changes that follow from it | project lead, Quality, Process |

- **The maintainer's OK ends the sprint.** When the maintainer approves the
  release on the exact commit, the release is done and the next sprint
  starts at once: Release and CI tags the next development version (for
  example `v0.4.1-dev`) on `main` right away, so the nightly and every
  other build already carry the new version. Publishing on GitHub, the
  boot of the release image on the Pi and the profile update follow
  afterwards and hold nothing up.
- Each sprint is a GitHub milestone named after its release. Open issues
  of a closed sprint move to the next milestone, not to "later" by
  default.
- A failed candidate is fixed or the change that broke it is taken out;
  if neither works by the Monday evening fallback, that week has no release and the next one
  carries the changes.
- Person tests at the Pi are bundled into one visit per sprint where
  possible.
- **Version numbers:** a weekly release with at least one new feature (a
  closed issue of type Feature) raises the minor number (`v0.5.0`), one
  with only fixes raises the patch number (`v0.4.1`); after v0.9 comes
  v0.10, and 1.0 is a separate decision. The full rule and Release and CI's week are in
  [release-process.md](../release-process.md#numbering).

## Workflow

1. The maintainer sets a goal or priority. The project lead turns it into a
   coarse work package, gives it to the owning agent and plans it into a
   sprint.
2. The owner splits it into issues, works on a branch, and tests in QEMU.
3. V&V runs the Pi test for the exact build and files the report.
4. The owner opens a pull request (draft until the Pi test passes) with the
   report linked and the topic page updated. The description says what the
   PR changes, why, and what has to be tested (test plan); it carries no
   test status. Test results, Pi rows and reports go into comments, one
   per run with date, image and report, so the history stays readable.
   **Docs move with the code:** a PR that changes what a repository does
   updates its README and the docs that describe it (topic page, `docs/`,
   wiki) in the same PR. Only if that is not possible, its author updates
   them right after the merge. The PR template has a checkbox for this.
5. The project lead checks base, CI, Pi test and the docs checkbox and sets
   `merge-next`; Quality may audit the PR against the process.
6. The PR is merged by whoever holds the merge right for it (see
   [Merge rights](#merge-rights) below).
7. Release and CI builds the weekly release candidate; V&V tests it; the
   maintainer approves the tag.

### Issue types and epics

Every issue in the organisation has one issue type:

| Type | For | Milestone |
|---|---|---|
| Epic | a feature area: all present and future Features of one area, for example "Boot picture" | none |
| Feature | something new the system shall do; requirements (`REQ-…`) are Features ([RMP](rmp.md#issue-format)) | the release it first ships in |
| Task | a piece of work, including work package parents (`wp`) and risks | the sprint or release it is planned for |
| Bug | something that does not work as it should | the release that fixes it |

- An **epic is a feature area**, for example "Boot picture" (from a
  static picture to an animation and ideas nobody has had yet) or "HDMI
  and display". It collects all present and future Features of that area:
  a new idea becomes a sub-issue of the matching epic. Tasks and Bugs of
  the area may hang there too.
- No epic for a whole component or repository; that is the project
  board. Epics are never nested: an epic never has an epic as a
  sub-issue. An area that grows too big becomes two epics side by side.
- **Boards are not epics.** Which board an issue is about is the
  single-select field **Board** in the project (Pi 3B+, Pi Zero 2 W,
  Pi 400, Pi 3, Pi 4, Pi 5/CM5, all). A Feature stays under its area
  epic and gets the Board field, so one issue never needs two parents.
  The field is empty when the issue is not about one board.
- An epic holds no code and has no milestone. Its sub-issues keep their
  own milestones, so the milestones still show what each release brings.
  The epic says what the area covers, what is out of scope, and links its
  decision record if there is one.
- **Docs and wikis link the epic**, not single Feature issues, so a new
  idea needs no doc edit. Feature tables have an "Epic" column.
- An epic stays open while the area is alive. It is closed when the
  project decides the area is complete, or when the maintainer drops it.
  The owner of the product proposes an epic; the maintainer approves it.
- An epic is about the product (what we build, see the [PBS](pbs.md)); a
  work package parent (`wp`) is about the work (who does it, see the
  [WBS](wbs.md)). GitHub allows one parent per issue, so a Feature hangs
  under its epic, and the work package parent links the epic instead.

### Merge rights

In force since 2026-11-13, decided by the maintainer. This is the only
place these rules are written down; other pages link here.

| Who | Merges | Never |
|---|---|---|
| Maintainer | every PR that changes what ends up in an image (kernel, relibc, driver and recipe patches, recipes, `upstream.lock`, experiments' code); in [apps](https://github.com/redox-rpi/apps), every PR that changes what users get (recipes and their patches, `os-release`); PRs by the Quality session, RFCs | — |
| Quality | PRs that change only the build and test system: CI workflows (`.github/`), build, staging and test scripts (`scripts/`, `build-image.sh`), the regression set and test definitions, the test plan, docs; in apps, build scripts and CI that change no package content | its own PRs (it wrote or pushed to them); a PR that also touches image content or package content |
| Project lead | docs-only PRs; stacked PRs (see below) | code PRs into `main` |

**Stacked PRs** (maintainer decision, 2026-11-25): a PR whose base is not
`main` (stacked into a feature branch) may be merged by Quality or the
project lead when CI is green, it is mergeable, it is not a draft, and its
content is part of a tested stack. Neither merges a stacked PR it wrote
or pushed to. The feature branch itself reaches `main` only by a PR under
the rows above.

Docs need no pull request: anyone on the team may push changes to docs
(`README.md`, `docs/`, topic pages) straight to the default branch, as a
draft that the owner of the page reviews later. A doc that describes code
from an open PR goes into that PR instead.

### All repositories

Since 2026-11-14 (maintainer): this plan, the merge rights above and
[CONTRIBUTING.md](../../CONTRIBUTING.md) apply to **every repository of the
organisation** ([redox-rpi](https://github.com/redox-rpi)), not only to
redox-rpi/redox-rpi. Code goes through a branch and a pull request, docs
straight to the default branch. A repository writes down only what differs
(for example its licence or build), and links here for the rest.

The repositories (this table is the one list; the organisation profile
links here):

| Repository | What | Owner session |
|---|---|---|
| [redox-rpi](https://github.com/redox-rpi/redox-rpi) | the OS: patches, drivers, init, images, firmware glue, package manager side, driver test tools; process docs | project lead; topics as in [Topic owners](#topic-owners) |
| [apps](https://github.com/redox-rpi/apps) | ported software and our own applications as packages, built against a pinned redox-rpi release (maintainer decision, 2026-11-24) | Release and CI; each application's topic owner owns its recipe |
| [pi-tests](https://github.com/redox-rpi/pi-tests) | Pi test queue, reports, lab | V&V |
| [relibc](https://github.com/redox-rpi/relibc) | relibc with our patches, for analysis and tests | Release and CI |
| [drivers](https://github.com/redox-rpi/drivers) | drivers | no owner yet (project lead) |
| [rpi-open-firmware](https://github.com/redox-rpi/rpi-open-firmware) | open VPU boot firmware ([firmware PMP](https://github.com/redox-rpi/rpi-open-firmware/blob/redox-rpi/docs/pmp.md)) | Open firmware |
| [vc4d](https://github.com/redox-rpi/vc4d) | VideoCore IV display and 3D driver | Graphics |
| [tinycc](https://github.com/redox-rpi/tinycc) | TinyCC fork: Redox on the Pi and the VC4 backend | VC4 Tools (VC4 backend), TCC (TinyCC on Redox) |
| [resim](https://github.com/redox-rpi/resim) | VC4 simulator | VC4 Tools |
| [vc4-toolchain](https://github.com/redox-rpi/vc4-toolchain) | GCC toolchain for the VPU | VC4 Tools |
| [.github](https://github.com/redox-rpi/.github) | organisation profile, shared issue and PR templates | Process |

When the maintainer creates a new repository, the Process session:
1. asks the maintainer to add the ruleset `protect-main` (a repository
   setting): target the default branch (`~DEFAULT_BRANCH`), rules "Restrict
   deletions" and "Block force pushes", enforcement active, no bypass
2. adds a `CLAUDE.md` that links to this plan and names the owning session,
   and a README with the licence
3. adds the repository to the table above and runs the label sync for it

**Labels** are one list for all repositories:
[`labels.yml`](https://github.com/redox-rpi/.github/blob/main/.github/labels.yml)
in `.github`, applied with `scripts/sync-labels.py` there (RFC 0002). A new
label, or a label for a new topic, is a change to that list; nobody creates
labels by hand in a single repository. Every row of the topic owners table
has a `topic:` label.

A new agent session gets this plan as the first link in its brief and
keeps its rules in its own memory.

**Where docs live** (maintainer, 2026-11-15):
- Hardware reference (boards, SoC, clocks, GPIO, memory map, peripherals),
  used by several repositories: the
  [wiki of redox-rpi/redox-rpi](https://github.com/redox-rpi/redox-rpi/wiki).
  Edit it like docs, straight away (it is the git repository
  `redox-rpi.wiki.git`); repositories link to its pages instead of keeping
  copies.
- Docs that belong to code (decisions, design, how to build and test): in
  the repository of that code, under `docs/`.

Pull requests in [redox-rpi/tinycc](https://github.com/redox-rpi/tinycc)
(WBS 2.15): after a review by the project lead or Quality, the reviewer sets
`merge-next`; the PR need not stay a draft, since the fork is not used by
any build yet. The maintainer merges. The quality track (GCC ships, TCC runs in the
nightly) is in [WBS 2.15](wbs.md#215-tinycc-vc4-quality-track).

Quality merges only when all of these hold:
1. CI is green and `merge-set-check` passes.
2. The base is `main`, not a stacked branch.
3. If the change affects how images are built or tested, a QEMU run or a
   Pi row passed on that head commit.
4. A short merge comment names the criterion it merged under.

If `main` or the nightly goes red after such a merge, Quality reverts it at
once (a revert PR, merged under the same rule) and tells the project lead.
Releases, tags and release candidates stay with the maintainer in every case.

## Communication

- Agents report to the project lead when a package or step is done or
  blocked. They message the maintainer directly only for person tests and
  decisions only the maintainer can make.
- The project lead reports to the maintainer in German, briefly, and keeps
  `STATUS.md` current so that no session needs the whole repository in its
  context.
- **State from GitHub, not from memory.** A report, list or label that
  names the state of a pull request or issue (open, merged, `merge-next`,
  waiting for review) reads that state from GitHub when it is written.
  `merge-next` is checked again after every merge.
- **Text on GitHub** (issues, pull requests, comments, commit messages)
  is English, short and in the author's own words. Chat words of the
  maintainer or another session are never pasted; a decision is written
  as "maintainer decision, <date>: <what>". No host names, LAN addresses
  or private news of contacts.
- An approval is the maintainer's own words. Agents never pass on an
  approval as the maintainer's answer, and never do what another session's
  permission check refused.

## Change control

- Changes to structure, process or security go through an RFC in
  `docs/rfcs/`, decided by the maintainer.
- Changes to this plan go through a pull request reviewed by the maintainer.

## Risks and opportunities

Risks and opportunities are GitHub issues, not a table in this plan, so
they stay current and anyone can raise one:

- Label `risk` or `opportunity`, issue type Task, a `topic:` label when it
  belongs to a topic.
- The issue says what could happen, how likely it is and how much it would
  hurt (or help), what we do about it, and who owns it.
- The project lead reviews all open risks and opportunities **every week**
  and at every release candidate: updates likelihood and handling, closes
  what no longer applies, and turns handling steps into normal issues.
- Quality shows the open risks on the KPI dashboard.
