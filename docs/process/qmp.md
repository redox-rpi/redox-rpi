# Quality management plan (QMP)

How redox-rpi keeps its quality in view: the goals, the process checks,
how bugs are analysed, the KPIs, when they are reviewed and how deviations
reach the project lead. The project management plan
([pmp.md](pmp.md)) defines the roles and the standards documents; this
plan covers quality. Owner: the Quality agent. Changes go through a pull
request, like any other document.

## Roles

The roles are defined in the [PMP](pmp.md#roles). Their part in quality:

| Role | Quality tasks |
|---|---|
| Maintainer | decides on deviations the lead escalates; approves this plan |
| Project lead | receives the quality reports, acts on deviations, sets `merge-next` (checked by P3) |
| Quality | this plan: process checks, bug analysis, KPIs and the dashboard; audits test coverage and states gaps as requirements on V&V. Does not write or run tests |
| V&V | writes and runs the tests (test standards, Pi rows, nightly checks, regression set) |
| Requirements | requirement IDs and their trace; the base of the coverage KPI |
| Architect | coding standards, including issue references in code (checked by P8) |
| Release and CI | CI, nightly, release candidates and checklists |
| Topic owners | root cause and fix of the bugs in their topic |

Quality never fixes another topic's bug: it files or comments on the
issue, names the owner, and tells the lead.

## Quality goals

1. **A green main.** Every commit on `main` passes CI; a red main is seen
   and fixed within a working day.
2. **Nothing reaches main or a release untested.** Features go to `main`
   only after a Pi test ([CONTRIBUTING.md](../../CONTRIBUTING.md#branches));
   a release candidate comes from a green commit and runs its whole
   checklist.
3. **Known bugs are known.** Every recurring or flaky bug has an issue, an
   owner, a root-cause status and a sighting count; a bug that is closed
   but still seen is reopened or listed as known in the release notes.
4. **Traceable work.** Every PR names its issue, every workaround in a
   patch names the bug it works around, every test result names its build
   ID.
5. **Measured, not felt.** The KPIs below are collected by a script and
   reviewed on a fixed cadence.

## Process checks

Quality audits these rules (P1–P11), from the repository and GitHub, without
blocking anyone: findings go to the lead (see [Deviations](#deviations)).
The rules themselves live where they are listed; this plan only checks them.

| # | Rule | Source of the rule | How it is checked |
|---|---|---|---|
| P1 | A PR names its issue (`Fixes #N`, `refs #N`) | CONTRIBUTING, "Topics and issues" | KPI script: merged PRs without an issue reference |
| P2 | A feature reaches `main` only after a Pi test | CONTRIBUTING, "Branches"; test-plan rule 6 | KPI script: merged PRs that change `patches/`, `tools/` or `config/` without a Pi test or row named in the PR |
| P3 | CI is green and current at merge | this plan (goal 1) | KPI script: main's red hours; PRs merged on CI from before a patch change on main (#81 → [#257](https://github.com/redox-rpi/redox-rpi/issues/257)) |
| P4 | A release candidate comes from a green commit and runs its whole checklist | release-process.md | per candidate, by hand: CI run of the commit, checklist results |
| P5 | Branches reach `main` only by PR; the user merges code PRs | CONTRIBUTING, "Merging" | KPI script: commits on `main` that change `patches/` outside a PR merge |
| P6 | Issues carry `topic:<name>`; `in-progress` issues are updated | CONTRIBUTING, "Topics and issues" | KPI script: open issues without a topic label; `in-progress` with no update in 14 days |
| P7 | The topic page changes with its topic | CONTRIBUTING; `scripts/check-topics.py` | the CI warning; weekly look at the warnings on main |
| P8 | A bug fix or workaround names its issue in the code and in the patch header (`// Workaround for #92: …`, `fixes #92`) | PMP, "What the standards must cover"; coding standards | KPI script: default patches whose header names an issue; code comments once the coding standards are in |
| P9 | No `--allow-dirty` results; results name the build ID | CONTRIBUTING, "Rules" | Pi rows on the dashboard; `-dirty` build IDs in done rows |
| P10 | Test-plan rows: unique numbers, a report for each done row | test-plan.md, "Rules" | by hand at the weekly review |
| P11 | The trace requirement → issue → PR → code → Pi row → release notes is complete | PMP, "Traceability" | KPI script, once requirements exist |

## Bug analysis

Quality analyses bugs together with the topic owner: it classifies them,
shows the pattern and the recurrence, and asks V&V for a test. The owner
finds and fixes the cause.

**Classification** (on the issue, and in the known-issues table of
[quality.md](../topics/quality.md#known-issues)):

- **Severity:** S1 the Pi crashes, hangs, loses data, or main or a release
  is broken; S2 a feature fails, or a workaround is needed; S3 noise,
  cosmetic, slow. Proposed labels `severity:S1`–`S3` (the lead decides).
- **Kind:** crash/panic, hang, data loss, wrong result, performance,
  build or process.
- **Recurrence:** one-off, recurring (seen in more than one row or
  release), flaky (comes and goes on the same image; give the rate, e.g.
  19 of 20).
- **Root-cause status:** unknown → suspected → confirmed → fixed in an
  experiment → fixed on `main` → in a release. "Closed" on GitHub is not
  the same as "fixed in the image" (#108).

**Root-cause patterns** that came back so far, to check first:

| Pattern | Examples |
|---|---|
| Merge order or stale CI: each change green alone, broken together | base `0083` fuzz after #81 (2026-11-07) |
| Race or timing at boot | #217, #79 |
| Long-run resource exhaustion or stall | #92, #129, #119, #151 |
| Upstream behaviour (netstack, relibc) | #140, #141, #205 |
| Hardware detail QEMU does not have | half-rate PWM audio (row 279), #151 |
| Fix only in an experiment, issue closed | #108 |

**Recurrence:** every Pi row and nightly that shows a tracked bug is a
sighting. The KPI script counts the done rows that name the issue; the
table in quality.md keeps the last sighting and the rate. A bug seen again
after it was closed is reopened (or a new issue that links the old one).

**Asking for a test:** when a bug has no check that would see it come
back, Quality writes the requirement (what to detect, how often, pass
criterion) on the issue for V&V.

## KPIs

Collected by [`scripts/quality-kpis.py`](../../scripts/quality-kpis.py)
over the last 30 days unless named otherwise; the page is
[kpis.md](kpis.md). Targets are a first proposal for the lead.

| KPI | Definition | Data source | Target |
|---|---|---|---|
| Open bugs by severity | open issues labelled `bug`, by `severity:` label; "unclassified" when none | GitHub issues | S1: 0 older than 7 days |
| Unlabelled issues | open issues without a `topic:` label | GitHub issues | 0 |
| Stale in-progress | open `in-progress` issues with no update in 14 days | GitHub issues | 0 |
| Reopened bugs | issues reopened in the window | GitHub timeline | tracked, no target |
| PR lead time | median and 90th percentile of opened → merged, merged PRs | GitHub PRs | median under 3 days |
| PRs without an issue | merged PRs with no issue reference (P1) | GitHub PRs | 0 |
| Code PRs without a Pi test | merged PRs changing `patches/`, `tools/`, `config/` with no Pi test or row named (P2) | GitHub PRs | 0 |
| Stale-CI merges | merged PRs whose last commit (so their last CI run) is older than the merge of an earlier PR in the same patch directory (`patches/base`, `patches/kernel`, …) or `config/`: the two were never built together (P3) | GitHub PRs | 0 |
| Main red hours | hours `main`'s `QEMU boot test` was red | GitHub Actions | under 8 h per red run of commits |
| Nightly pass rate | nights that made a new stable image / nights that built (fail = a report from `update-stable.sh` on the issue "Nightly: main fails on the Pi") | git log of main, GitHub issue | 100 % |
| Pi pass rate | done test-plan rows whose result starts with "pass" / all done rows | `docs/test-plan.md` "Done" | tracked; a fail needs an issue |
| Flaky rate | done rows that name a tracked recurring bug / all done rows, and the sightings per bug | `docs/test-plan.md` "Done" | falling |
| Patches with an issue | default patches whose header names an issue (P8) | `patches/` | rising |
| Dirty results | done rows with a `-dirty` build ID (P9) | `docs/test-plan.md` | 0 for merges and releases |
| Release checklist completion | checklist tests with a result / all tests, per candidate | release checklist | 100 % before a release; **not measurable yet**: the checklist has no result column |
| Test coverage per requirement | requirements (`REQ-<area>-<n>`) with a passing test / all requirements | `docs/requirements/`, once they exist (PMP) | to be defined |
| Trace completeness (P11) | requirements with issue, PR and test / all requirements | `docs/requirements/`, GitHub | to be defined |

## Review cadence

| When | What | Output |
|---|---|---|
| Weekly, Monday after the upstream jobs | KPI page regenerated; process checks P1–P11; known-issues table | short report to the lead; issues for new deviations |
| Every release candidate | quality gate: P4, checklist completion, open S1 bugs, known issues in the notes | go / no-go note on the release issue for the lead and the user |
| After each S1 bug or red main | pattern check (table above), test requirement for V&V | comment on the issue |
| Monthly | this plan: KPIs, targets, rules that no longer fit | PR to this file |

## Deviations

A deviation is a broken rule (P1–P11), a KPI outside its target, or an S1
bug without an owner. Quality:

1. writes it down with the evidence (commit, PR, row, KPI value) as an
   issue with `topic:quality` or as a comment on the existing issue, and
   names the owner from the PMP;
2. reports to the lead in a few lines: what, since when, owner, proposal;
3. **at once** for a red main, a release candidate that breaks P4, or a
   new S1 bug; otherwise in the weekly report;
4. tracks it until it is closed; a deviation open after two weekly reports
   goes to the lead again, and the lead decides whether the user needs to
   know.

Quality reports and proposes; it does not merge, revert or reassign work.
