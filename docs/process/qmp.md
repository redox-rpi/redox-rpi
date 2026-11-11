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
| P2 | A feature reaches `main` only after a Pi test | CONTRIBUTING, "Branches"; queue rule 6 in [pi-tests queue.md](https://github.com/redox-rpi/pi-tests/blob/main/queue.md) | KPI script: merged PRs that change `patches/`, `tools/` or `config/` without a Pi test or row named in the PR |
| P3 | CI is green and current at merge | this plan (goal 1) | KPI script: main's red hours; PRs merged on CI from before a patch change on main (#81 → [#257](https://github.com/redox-rpi/redox-rpi/issues/257)) |
| P4 | A release candidate comes from a green commit and runs its whole checklist | release-process.md | per candidate, by hand: CI run of the commit, checklist results |
| P5 | Branches reach `main` only by PR; the user merges code PRs | CONTRIBUTING, "Merging" | KPI script: commits on `main` that change `patches/` outside a PR merge |
| P6 | Issues carry `topic:<name>`; `in-progress` issues are updated | CONTRIBUTING, "Topics and issues" | KPI script: open issues without a topic label; `in-progress` with no update in 14 days |
| P7 | The topic page changes with its topic | CONTRIBUTING; `scripts/check-topics.py` | the CI warning; weekly look at the warnings on main |
| P8 | A bug fix or workaround names its issue in the code and in the patch header (`// Workaround for #92: …`, `fixes #92`) | PMP, "What the standards must cover"; coding standards | KPI script: default patches whose header names an issue; code comments once the coding standards are in |
| P9 | No `--allow-dirty` results; results name the build ID | CONTRIBUTING, "Rules" | Pi rows on the dashboard; `-dirty` build IDs in done rows |
| P10 | Queue rows: unique numbers, a report (`reports/<row>.md`) for each done row | [pi-tests queue.md](https://github.com/redox-rpi/pi-tests/blob/main/queue.md), "Rules" | by hand at the weekly review |
| P11 | The trace requirement → issue → PR → code → Pi row → release notes is complete | PMP, "Traceability" | KPI script, once requirements exist |

## Bug analysis

Quality analyses bugs together with the topic owner: it classifies them,
shows the pattern and the recurrence, and asks V&V for a test. The owner
finds and fixes the cause.

**Classification** (on the issue, and in the known-issues table of
[quality.md](../topics/quality.md#known-issues)):

- **Severity** (labels, decided by the lead on 2026-11-10):
  `severity:S1` blocks boot or a release, or loses data;
  `severity:S2` a feature is broken, a workaround exists;
  `severity:S3` minor or cosmetic (noise, slowness, a fix that waits
  only for upstream). Every open issue of type Bug has exactly one.
  Quality sets it at the weekly review for bugs that have none; the
  topic owner may change it with a reason in a comment. A new S1 is
  reported to the lead at once (see [Deviations](#deviations)).
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
| Open issues by repository and type | open issues per redox-rpi repository and GitHub issue type (Task, Bug, Feature) | GitHub issues of all organisation repositories | tracked |
| Issues without a type | open issues with no issue type | GitHub issues, all repositories | 0 once the types are rolled out (RFC 0002) |
| Open bugs by severity | open issues of type Bug or labelled `bug`, in all repositories, by `severity:` label; "unclassified" when none | GitHub issues, all repositories | S1: 0 older than 7 days |
| Unlabelled issues | open issues without a `topic:` label, except those labelled `risk` or `opportunity` (project-level, in the risk register) | GitHub issues | 0 |
| Stale in-progress | open `in-progress` issues with no update in 14 days | GitHub issues | 0 |
| Reopened bugs | issues reopened in the window | GitHub timeline | tracked, no target |
| PR lead time | median and 90th percentile of opened → merged, merged PRs | GitHub PRs | median under 3 days |
| PRs without an issue | merged PRs with no issue reference (P1) | GitHub PRs | 0 |
| Code PRs without a Pi test | merged PRs changing `patches/`, `tools/`, `config/` with no Pi test or row named (P2) | GitHub PRs | 0 |
| Stale-CI merges | merged PRs whose last commit (so their last CI run) is older than the merge of an earlier PR in the same patch directory (`patches/base`, `patches/kernel`, …) or `config/`: the two were never built together (P3) | GitHub PRs | 0 |
| Main red hours | hours `main`'s `QEMU boot test` was red | GitHub Actions | under 8 h per red run of commits |
| Nightly pass rate | nights that made a new stable image (its commit "Nightly: … is the stable image", in pi-tests since 2026-11-09) / nights that built (fail = a report from `update-stable.sh` on the issue "Nightly: main fails on the Pi") | git log of main and of redox-rpi/pi-tests, GitHub issue | 100 % |
| Pi pass rate | done queue rows whose result starts with "pass" / all done rows | [pi-tests queue.md](https://github.com/redox-rpi/pi-tests/blob/main/queue.md) "Done" | tracked; a fail needs an issue |
| Flaky rate | done rows that name a tracked recurring bug / all done rows, and the sightings per bug | [pi-tests queue.md](https://github.com/redox-rpi/pi-tests/blob/main/queue.md) "Done" | falling |
| Patches with an issue | default patches whose header names an issue (P8) | `patches/` | rising |
| Dirty results | done rows with a `-dirty` build ID (P9) | [pi-tests queue.md](https://github.com/redox-rpi/pi-tests/blob/main/queue.md) "Done" | 0 for merges and releases |
| Network losses in soak (#92) | losses (see [Network loss](#network-loss-92-217)) / soak hours, with the number of soaks and the 95 % upper bound when 0 | pi-tests `network.jsonl`, kind `soak` | a fix of #92 counts as confirmed when 0 losses in at least 20 soak hours |
| Boots without network (#217) | boots with no devsh login within 120 s after the login prompt / all recorded boots, with the 95 % upper bound when 0 | pi-tests `network.jsonl`, kind `boot` | 0; a fix counts as confirmed when the upper bound is below the old rate (1 in 1000 boots: about 3000 boots) |
| Seconds to devsh | median and 90th percentile of the seconds from the login prompt to the first devsh login, recorded boots (trend for #47) | pi-tests `network.jsonl`, kind `boot` | tracked, no target |
| Release checklist completion | checklist tests with a result / all tests, per candidate | release checklist | 100 % before a release; **not measurable yet**: the checklist has no result column |
| Test coverage per requirement | requirements whose Verification names a test with no **gap** / all requirements, by priority; `must` with a gap; failing; verified only by a person or a dirty build ([RMP](rmp.md#traceability) KPIs) | `docs/requirements/` (Feature issues after RFC 0002), `--req-rev` while in review | every `must` and `should` of the release has a test (RMP rule 2); 0 `must` with a gap |
| Trace completeness (P11) | requirements with an implementing issue (or "upstream") / all; REQ IDs in queue rows that do not exist (RMP rule 5) | `docs/requirements/`, pi-tests `queue.md` | 100 %; 0 unknown IDs |
| Feature coverage | release features (the checklist's "In the image since" table) with an automated check or a named person test / all release features | release checklist; by hand until the checklist maps feature → test | 100 % for release features |
| Pi rows per topic | done queue rows that name an issue of the topic | [pi-tests queue.md](https://github.com/redox-rpi/pi-tests/blob/main/queue.md), issue labels | every topic with code in the image has rows in the window |
| Host tests | our Rust crates (`tools/`, vc4d) with host unit tests / all such crates | the repository | see [Coverage](#coverage) |
| Code coverage | lines covered by host tests in host-testable crates (`cargo llvm-cov`) | build host, once measured | see [Coverage](#coverage) |
| Static analysis findings | warnings per check of [Code quality](#code-quality), against the baseline | [code-quality-baseline.md](code-quality-baseline.md), later CI | not above the baseline; see targets there |

### Network loss (#92, #217)

R3 of [quality.md](../topics/quality.md#requirements-on-vv) ([#259](https://github.com/redox-rpi/redox-rpi/issues/259)):
the network loss of #92 and the boots without network of #217 get a
rate, not sightings by chance. Quality defines what is counted; the
nightly (Release & CI) and V&V rows record it; `quality-kpis.py` sums it.

**Network loss:** during a soak, 3 pings in a row (10 s apart) without an
answer, or 2 devsh logins in a row that fail. It lasts until a ping and a
devsh login both work again; that is one loss, however long. **Boot
without network:** no devsh login within 120 s after `redox login:` on
the serial console.

**Record:** `network.jsonl` in the root of
[pi-tests](https://github.com/redox-rpi/pi-tests), one JSON object per
line, only appended. Every record has `date` (UTC, `YYYY-MM-DD`),
`build` (build ID), `source` (`nightly` or `row <ID>`) and `kind`:

- `boot`, one per boot (the nightly's reset, the reboot of #258, every
  boot of a row that waits for devsh): `devsh_s`, seconds from the login
  prompt to the first devsh login, `null` when none within 120 s;
  optional `login_s`, seconds from the reset to the login prompt.
- `soak`, one per soak: `minutes`, `pings`, `pings_lost`, `logins`,
  `logins_failed` and `losses`, a list with one `{"at_s": …, "down_s": …}`
  per loss (seconds since the soak's start; `down_s` is `null` when the
  network was not back by the end).

```json
{"date": "2026-11-12", "build": "v0.4.0-dev-700", "source": "nightly", "kind": "boot", "devsh_s": 21.5, "login_s": 48.0}
{"date": "2026-11-12", "build": "v0.4.0-dev-700", "source": "nightly", "kind": "soak", "minutes": 15, "pings": 90, "pings_lost": 0, "logins": 15, "logins_failed": 0, "losses": []}
```

The serial lines around a loss go into the night's report, not into the
record. kpis.md shows the KPIs above and a weekly table; a line that
cannot be read is counted and shown, never dropped silently.

**Enough data:** with 0 events in N trials, the 95 % upper bound of the
rate is about 3/N (rule of three). One nightly gives about 2 boots and
0.25 soak hours, so #217 needs V&V's boot loops (#266) for a rate, and a
"0" is never read as "fixed" before the bound is below the old rate.

## Test strategy

Which area is tested at which level. The levels are those of the test
standards ([test-standards.md](test-standards.md#test-levels), V&V):
1 host unit test, 2 QEMU, 3 CI, 4 real Pi automated, 5 person test; plus
**U** for checks that come from upstream Redox (their CI, the relibc test
suite). V&V owns the standards and runs the tests; Quality keeps this
matrix complete and measurable and audits it against the
[coverage KPIs](#kpis). Agreed with V&V on 2026-11-08
([#267](https://github.com/redox-rpi/redox-rpi/issues/267), review on
[PR #262](https://github.com/redox-rpi/redox-rpi/pull/262#issuecomment-6719908584));
the test standards link here.

● exists today (at level 4 "rows" means by hand in Pi rows, not in the
nightly), ○ a gap with who takes it, – not useful at that level. Gap
owners: **RS** V&V regression set ([#265](https://github.com/redox-rpi/redox-rpi/issues/265)),
**ST** V&V special tests ([#266](https://github.com/redox-rpi/redox-rpi/issues/266)),
**RC** Release & CI through the lead, **OW** the topic owner (host unit
tests), **acc** accepted gap.

| Area | 1 Host | 2 QEMU | 3 CI | 4 Pi auto | 5 Person | U |
|---|---|---|---|---|---|---|
| Patches apply, drift, reproducible build | – | – | ● check-patches, drift weekly; ○ CI current at merge (#257, RC) | – | – | – |
| Kernel: boot, SMP, scheduler, memory | – | ● | ● kernel only, upstream userspace; ○ our full image (#260, RC) | ● nightly starts | – | ● |
| Kernel: signals, pipes, start crashes | ○ sigregs-like unit checks (OW) | ● qemu-run start loop | ● sigregs on Linux | ● 300/1000 starts | – | – |
| relibc | ○ (OW) | ● relibc-tests by hand | ○ relibc suite in CI (#13, RC) | ● release 15 | – | ● suite |
| Boot chain: firmware, U-Boot, bootloader, netboot | – | ● QEMU boot | ● | ● every row boots; boot time from boot-timeline.py in benchmark rows | ● SD boot, boot menu (6) | – |
| SD card, RedoxFS | ○ redoxfs-file (OW) | ● `--data` | – | ● one file nightly, sd-stress release | ● root on the card (6) | ● redoxfs |
| USB: dwc2d, keyboard, mouse, storage, hotplug | – | ● usb-net in QEMU | – | ○ hub and storage without a person (acc: needs plugging) | ● 7, 8, 21 | – |
| Network: LAN7800, RTL8153, netstack, dhcpd, devshd | – | ● usb-net | ● QEMU usb-net boot | ● rows: TCP transfer (bench.py, rows 260, 289), boots without devshd in reboot loops (276, 285); ○ TCP in the nightly (#258, RS); ○ soak and boot count nightly (#259, ST) | ● Zero 2 W hub (9) | – |
| Time: timesync, timerd, time zones | ● timerd, timesync tests | ● | – | ○ NTP check nightly (#258, RS) | – | – |
| Firmware interface: clock, governor, reboot, poweroff | ● socdump tests | ● cpu-ondemand qemu-check | – | ● release 2, 5, 13; ○ nightly (#258, RS) | – | – |
| GPIO, I2C, SPI | ○ protocol logic (OW) | – | – | ● no-wire check (release 14) | ● wired rows | – |
| Graphics: rpi-fbd, vc4d, desktop | ○ vc4d logic, tests do not build on a host (#271, OW) | ● fb in QEMU | – | ● serial rows (rpi-fbd), desktop without a person (row 288), vc4shot (276); ○ scripted desktop smoke and console regression (#261, RS) | ● screen visits, 22 | – |
| Audio: PWM driver, audiod, tone | ○ ring and stats logic (OW) | ● qemu-check.sh by hand | ○ (RC) | ● real rate from tone stats (rows 281, 283, 284), release 16; ○ nightly rate (#258, RS) | ● listening (17) | – |
| Serial console, getty | – | ● | – | ● serial-input-test (11) | – | – |
| Tools: assistant, socdump, cputop, welcome | ● 6 of 20 crates; more is OW | ● | ○ cargo test (#273, RC) | – | – | – |
| Host scripts: pi-test, publish, KPI | ○ parsers (OW) | – | ● topics check | ● used every night | – | – |
| Zero 2 W | – | – | – | ● once (row 206); ○ no second board on the network (acc) | ● 9 | – |
| TCC (native C compiler) | – | ○ (OW) | – | ● rows 278, 282 | – | – |
| Robustness: chaos, start and reboot loops, soak | – | ● start loop | – | ● chaos by hand (#196), 1000 starts and reboot loops for releases; ○ soak (#266, ST) | – | – |
| Release image: download, checksum, notes | – | ● rc in QEMU | – | ● release 1–5 | ● visit | – |

Rules for the matrix:

- A release feature without a ● at level 4 or 5 is a gap of feature
  coverage and blocks the release unless the lead accepts it.
- Every ○ is either a requirement on V&V (with an issue) or accepted as a
  gap here, with the reason.
- When requirement IDs exist, each row gets its `REQ-…` IDs, and the KPI
  "Test coverage per requirement" replaces the area view.

## Coverage

Three kinds, each a KPI:

| Kind | What counts | Today (2026-11-08) | Target |
|---|---|---|---|
| Feature / requirement coverage | each release feature has an automated check (level 2–4) or a named person test (5) | v0.4.0: 8 of 12 image features (67 %); none for multicast (#27), TCP close (#88), bootloader skip (#232), socdump (#224); V&V writes checklist steps for the first three, socdump is a person test (open firmware card) | 100 % of release features from rc2 on; per requirement once `REQ-…` exist |
| Code coverage | lines of host-testable code run by host tests (`cargo llvm-cov`; our crates in `tools/`, the logic parts of vc4d and drivers, Python parsers) | not measured; 6 of 20 tool crates have tests (30 tests); vc4d's 6 tests do not build on a host | step 1 (v0.5): measured in CI report-only, every crate with pure logic has tests; step 2: 50 % of host-testable lines, 70 % for new code in a PR |
| Pi row coverage per topic | done rows per topic in 30 days, and topics with code in the image but no nightly check | see [kpis.md](kpis.md) | every topic in the image has a nightly or release check |

Code coverage is only measured where host tests are possible. Drivers and
kernel code are covered by levels 2–5, not by line coverage.

## Code quality

Static analysis of our own code, report-only first. Baseline and details:
[code-quality-baseline.md](code-quality-baseline.md).

| Check | Scope | Baseline (2026-11-08) | Target | CI proposal |
|---|---|---|---|---|
| clippy (default lints) | `tools/*` crates | 12 warnings in 6 crates; 2 crates do not build on a host | 0 warnings | report-only now, gating once 0 |
| clippy (Redox target) | our drivers in base (`scripts/clippy-redox.py`, #272) | 122 warnings, 54 undocumented `unsafe` in 8 crates (2026-11-11; sdhcid, upstream: 66); kernel, relibc patches and vc4d not yet | measured each night; no rise | report-only on the build host after the nightly |
| `unsafe` without a `SAFETY:` comment | `tools/*` | 51 blocks in 11 crates | 0 in new code | report-only (`clippy::undocumented_unsafe_blocks`) |
| `unsafe` count | our Rust (tools, patches' added lines, vc4d) | 198 lines in patches and tools, 17 in vc4d | tracked, not rising without a reason | KPI page |
| shellcheck | 36 shell scripts | 1 error, 8 warnings, 23 notes, all checked: no real bug | 0 errors and warnings (with justified `disable` comments) | report-only, then gating |
| pyflakes / ruff (E, F, W, B) | 22 Python files | pyflakes 0; ruff 60, style only | pyflakes 0 (gating) | pyflakes gating now |
| cppcheck | 5 C files (tcc-bench, chaos) | 1 warning (unchecked malloc) | 0 | report-only |
| cargo-audit | 17 `Cargo.lock` (115 packages; mcast, net6, socdump have none) | 0 advisories | 0 | weekly, with the drift job |
| TODO/FIXME without an issue | all our code and added patch lines | 2 (experiments `ipv6`, `sshd`) | 0 | KPI page |

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
