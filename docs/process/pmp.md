# Project management plan

How the redox-rpi project is organised: who does what, which documents
exist, and how work moves from an idea to a release. This plan is owned by
the project lead and changed by pull request; the maintainer approves it.

Status: draft, 2026-11-08.

## Goal

Redox OS on the Raspberry Pi 3B+ (and later Zero 2 W, Pi 4/5), with
reproducible builds, tested on a real Pi, released as versioned SD card
images. Milestones are the GitHub milestones `v0.x.0`; the roadmap is in
[status.md](../status.md).

## Roles

The project is run by one person, the **maintainer**, and a team of Claude
agent sessions. Each agent owns one area and keeps its knowledge in its own
memory, so the team stays stable; new agents are added only for expertise
nobody has yet.

| Role | Who | Owns |
|---|---|---|
| Maintainer | the user | goals and priorities, merging code PRs, releases and tags, RFC decisions, posts to the Redox community, tests that need a person at the Pi |
| Project lead | coordinator session | this plan, the coarse work packages and who owns them, overall status (`STATUS.md`), merge readiness (`merge-next`), docs-only merges, reporting to the maintainer |
| Quality | Quality session | the [QMP](qmp.md): process compliance, bug analysis, KPIs and the KPI dashboard. Quality does not test |
| Verification and validation (V&V) | Pi test agent, extended | test standards, test plan and reports, automated and special tests on QEMU and the real Pi, the regression set, preparing person tests |
| Requirements | new agent (proposed) | the requirements, their IDs and their trace to issues, code and tests |
| Architect | Architect session | architecture, coding standards and coding rules, RFC drafts, cross-topic technical decisions |
| Release and CI | Release & CI session | builds, CI, release candidates and releases, the image server |
| Topic owners | one session per topic | their topic: design, code, bug fixes, topic page. Today: Graphics (framebuffer driver, vc4d), Boot and console, Network, Audio, Open firmware, TCC, Log triage |

A topic owner breaks its coarse package into smaller work packages itself
(GitHub issues with the `topic:<name>` label), because it knows the area
best. It fixes the bugs in its area. Example: Graphics owns the new display
driver `vc4d` and splits it into packages, and keeps the framebuffer driver
`rpi-fbd`, including its bugs.

## Documents

| Document | Path | Owner | Status |
|---|---|---|---|
| Project management plan | `docs/process/pmp.md` | project lead | this draft |
| Quality management plan | `docs/process/qmp.md` | Quality | to write |
| Requirements | `docs/requirements/` | Requirements | to write |
| Architecture | `docs/architecture.md` | Architect | exists |
| Coding standards and rules | `docs/process/coding-standards.md` | Architect | to write |
| Test standards | `docs/process/test-standards.md` | V&V | to write |
| Test plan and reports | `docs/test-plan.md`, `docs/test-reports.md` | V&V | exist; to split (RFC 0002) |
| Release process and checklists | `docs/release-process.md`, `docs/releases/` | Release and CI | exist |
| Topic pages | `docs/topics/` | topic owners | exist |
| Contribution rules | `CONTRIBUTING.md` | project lead | exists; links the documents above |
| Overall status | `STATUS.md` | project lead | to write (RFC 0002) |

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

## Traceability

Requirement (`REQ-…`) → issue → branch and PR (references the issue) → code
(comment with the issue) → Pi row and report (names the requirement or
issue) → release notes. Quality checks that the chain is complete; the KPI
dashboard shows the gaps.

## Workflow

1. The maintainer sets a goal or priority. The project lead turns it into a
   coarse work package and gives it to the owning agent.
2. The owner splits it into issues, works on a branch, and tests in QEMU.
3. V&V runs the Pi test for the exact build and files the report.
4. The owner opens a pull request (draft until the Pi test passes) with the
   report linked and the topic page updated.
5. The project lead checks base, CI and Pi test and sets `merge-next`;
   Quality may audit the PR against the process.
6. The maintainer merges code PRs; the project lead may merge docs-only
   PRs. RFCs (`docs/rfcs/`) are always the maintainer's.
7. Release and CI builds release candidates; V&V tests them; the maintainer
   approves the tag.

## Communication

- Agents report to the project lead when a package or step is done or
  blocked. They message the maintainer directly only for person tests and
  decisions only the maintainer can make.
- The project lead reports to the maintainer in German, briefly, and keeps
  `STATUS.md` current so that no session needs the whole repository in its
  context.
- An approval is the maintainer's own words. Agents never pass on an
  approval as the maintainer's answer, and never do what another session's
  permission check refused.

## Change control

- Changes to structure, process or security go through an RFC in
  `docs/rfcs/`, decided by the maintainer.
- Changes to this plan go through a pull request reviewed by the maintainer.

## Risks

| Risk | Effect | Handling |
|---|---|---|
| The project no longer fits one context window | agents miss the status, decisions get lost | `STATUS.md`, split test docs, topic status blocks (RFC 0002) |
| One test Pi for all agents | queue waits, conflicts | V&V owns the Pi queue; one serial reader; later a test lab (topic boards) |
| Drift from upstream Redox | patches stop applying | `upstream.lock`, drift check, small patches |
| Flaky failures (#28, #92/#217) | releases with known gaps | Quality tracks them as KPIs; owners root-cause them |
| Expiring credentials (nightly token 2026-11-27) | nightly stops | the project lead reminds the maintainer a week before |

## Next steps

1. Quality writes the QMP and the first KPI dashboard.
2. The Pi test agent takes the V&V role and writes the test standards.
3. The Architect writes the coding standards and rules.
4. The maintainer decides on a Requirements agent; it then writes the first
   requirements for v0.4.
5. RFC 0002 (project structure) decided and migrated in small PRs.
