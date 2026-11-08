# RFC 0002: Project structure that fits a session's context

| | |
|---|---|
| Status | Draft |
| Created | 2026-11-08 |
| Issue | – |
| Releases | none; documentation and scripts only |

## Summary

No session can see the whole project any more. On 2026-11-07 the
architecture session did not know the full state of the project. The
cause is not the amount of code. It is that the files a session has to
read to learn the state are too large. The two test documents alone are
bigger than one context window.

This RFC proposes four changes:

1. A short, partly generated **STATUS.md** as the single entry point.
2. A **test plan** that holds only the open queue, in short rows.
3. **One file per Pi test** for its steps and its report.
4. A **status block** at the top of each topic page.

The code stays in one repository. Splitting it up would cost more than it
saves; the reasons are under [Repositories](#repositories).

## Motivation

### What a session reads today

Measured on main on 2026-11-08, at about 4 bytes per token:

| File | Size | Tokens | Who reads it |
|---|---|---|---|
| `docs/test-reports.md` | 610 KB | ~150k | every Pi test writes to it; it is read when a result is needed |
| `docs/test-plan.md` | 190 KB | ~47k | every session that queues, takes or checks a test; the coordinator every 10 minutes |
| ... of that, rules and the stable image | 10 KB | 2.5k | |
| ... of that, the queue (about 38 rows) | 76 KB | 19k | |
| ... of that, done rows (about 260) | 105 KB | 26k | |
| `docs/status.md` | 15 KB | 4k | nobody; last update 2026-10-24 |
| `docs/topics/graphics.md`, `assistant.md` | 35 KB, 32 KB | 9k, 8k | the topic's sessions |
| all 20 topic pages | ~210 KB | ~52k | |
| `ROADMAP.md` | 14 KB | 3.5k | coordinator, architecture |

To answer "where do we stand?", a session would have to read the queue,
the newest done rows, the open PRs, the topic pages and the stable image
block. No single file has that answer.

The open queue is the surprise. Its 38 rows are 76 KB, because each row
holds its full steps; row 271 alone is about 3 KB. Moving the done rows
out is not enough.

### Other effects of the same files

- **Merge conflicts.** 363 of the 589 commits on main since 2026-11-01
  touch `docs/test-plan.md`. Nearly every Pi test changes it three times:
  queue, on the Pi, done. `scripts/pr-order.py` already lists it as a file
  that "every PR touches".
- **Stale status.** `docs/status.md` was meant to be the status page. Nobody
  kept it current, because the real state lived elsewhere: queue, PRs,
  issues. A status page written by hand will go stale again unless the
  parts that change daily come from a script.

### What is not the problem

- **Code size.** `tools/` holds 24 programs with 800 KB in total. 21 of
  them are built into the image from `tools/*/recipe.toml`. Sessions read
  a tool only when they work on it.
- **Patches.** There are 147 patch files and 187 files under
  `patches/experiments`. Sessions look up the one they need, through
  `patches/README.md` and `config/upstream.lock`.

## Goals

- A new session learns the state of the whole project from one file of at
  most 8 KB (about 2k tokens).
- A session that works on one topic reads that file, the topic's status
  block and its own test files: under 20k tokens before it starts working.
- Writing a Pi test result no longer edits a file that every other PR
  edits too.
- Every script and CI job keeps working after each migration step.

## Non-goals

- No change to how images are built, versioned or tested.
- No move to a different tracker (GitHub Projects, a wiki). See
  [Alternatives](#alternatives-considered).
- No rewrite of old reports. They are archived as they are.

## Design

### 1. STATUS.md: the entry point

`STATUS.md` sits in the repository root, next to `README.md` and
`ROADMAP.md`, and replaces `docs/status.md`. It has two parts, separated by
markers:

**Generated part** (`scripts/status.py`, run by the coordinator and by the
nightly). Every fact in this part already lives somewhere a script can
read, so nobody has to keep it current:

- Release: newest tag, and commits on main since that tag.
- Stable image: the build ID, from the stable-image block.
- Pi: the test marked **on the Pi**, and the next three queue rows (number,
  title and owner only).
- Open PRs, from `gh pr list`: number, title, labels (`merge-next`, draft),
  and the CI state.
- Open issues per milestone: counts only, with a link.
- Linked sessions are left out: they live in the app, not in git.

**Hand-written part**, kept by the coordinator:

- One line per topic: state in a few words, the next step, and a link to
  its page.
- At most five "decisions waiting for the maintainer".

A CI warning fires when STATUS.md grows over 8 KB, the same way
`check-topics.py` warns about topic pages. Every session brief starts with
"read STATUS.md". `CONTRIBUTING.md` and the Pi agent's and coordinator's
instructions say the same.

The findings in today's `docs/status.md` move to the topic pages they
belong to, or to `docs/roadmap-history.md` when they are history.

### 2. The test plan holds only the open queue

`docs/test-plan.md` keeps:

- the rules,
- the stable-image block, which `update-stable.sh` writes,
- the queue.

A queue row becomes one short line, at most about 300 characters:

```
| 289 | PR #226 opt-a53 split (A, B2, C, D2) | Architecture | [steps](tests/0289.md) | network, serial | waiting |
```

The columns stay the same as today (`# | Test | Owner | Image | Needs |
Status`), so `pr-order.py`, `update-stable.sh` and `visit-index.py` keep
parsing them. The Image cell links to the test file. A row that a nightly
visit image belongs to keeps the text ``nightly visit `name` `` in its Test
cell, which is what `visit-index.py` and `sdcard-index.py` look for.

Done rows move to **`docs/test-done/YYYY-MM.md`**, one file per month,
with the same columns as today's Done table. The newest done rows are
found through the generated part of STATUS.md.

The coordination text (about 20 lines on what the coordinator does) moves
to `docs/testing.md`. That page already explains how testing works.

### 3. One file per Pi test

Each new queue row gets **`docs/tests/NNNN.md`**:

- top: the steps, the images with their build IDs, and the pass criteria.
  This is today's long queue row.
- bottom: the report. `pi-test.py --report docs/tests/NNNN.md` appends it,
  and the agent adds the tables.

Once the test is done, the file stays. Topic pages and PRs link to it
directly.

The row number is still the key. The number rule (highest number in the
file plus one) also counts `docs/tests/` and `docs/test-done/`, so a
number is never used twice.

The benefits:

- The agent reads one file for one test.
- Two tests no longer touch the same lines.
- A result can be found from its PR without searching 610 KB.

`docs/test-reports.md` is frozen as it is and moves to
`docs/test-reports/2026-10.md` and `2026-11.md`, split at the month line.
Results that are not Pi tests also get their own file: QEMU runs,
benchmarks, the nightly report from `update-stable.sh`. The nightly
writes `docs/tests/nightly/YYYY-MM-DD.md`. An old link to
`test-reports.md` lands on a short index page at that path, with one line
per month.

### 4. Status blocks on topic pages

Each `docs/topics/*.md` page starts with a block of at most 10 lines:

```
## Status (2026-11-08)
- State: stage 1 HVS on the Pi (PR #188); V3D not started.
- Next: ...
- Open: #188, #200 · Tests: [0287](../tests/0287.md)
- Owner: graphics session
```

Below it comes the rest, as today: plan, design, history. A session that
only needs the state reads the first 15 lines. The coordinator copies the
"State" line into STATUS.md. `check-topics.py` warns when a page has no
status block, or when its date is older than the newest change to the
topic's files.

### How scripts and CI keep working

| Script / job | What it reads today | After |
|---|---|---|
| `pr-order.py` | Queue and Done in `test-plan.md` | Queue there, Done from `docs/test-done/*.md`; `COMMON` gets the new paths |
| `update-stable.sh` | "on the Pi" in the queue, the stable-image block; appends to `test-reports.md` | the same; appends to `docs/tests/nightly/<date>.md` |
| `visit-index.py`, `sdcard-index.py`, `build-visit-images.sh`, `publish-sdcard.sh` | queue rows naming ``nightly visit `x` `` | unchanged: the text stays in the short row |
| `pi-test.py`, `sd-stress.py` | `--report docs/test-reports.md` in their help text | help text and examples name `docs/tests/NNNN.md` |
| `check-topics.py` (`topics.yml`) | `config/topics.map` | also: status block present and fresh; STATUS.md under 8 KB |
| `build-image.sh`, `with-build-lock.sh` | comments only | comments only |

## Repositories

### What stays in `redox-rpi` and why

The patches, `config/upstream.lock`, `config/`, `scripts/`, the image
tools in `tools/` and the docs stay together. A build ID (git describe
plus the experiments hash) names exactly one state of all of them, and
the manifest records it. If an image tool lived in its own repository:

- `upstream.lock` would need a line for it,
- a change would need two PRs, one in each repository,
- a test on the Pi would need both PRs merged in the right order.

That is the work `vc4d` already costs. It is worth it there, because vc4d
is a driver with its own life. It is not worth it for a 20 KB test
program.

Splitting the repository would also not save context: sessions do not
read `tools/` unless they work on a tool.

### What has, or may get, its own repository

| Part | Repository | Why |
|---|---|---|
| vc4d | `redox-rpi/vc4d` (exists) | driver with its own releases |
| open firmware | `redox-rpi/rpi-open-firmware` (exists) | different toolchain (VC4) |
| relibc changes | `redox-rpi/relibc` (exists) | for upstream PRs |
| `tools/rpi-boot-stub` | candidate: into `rpi-open-firmware` | host-built boot code; belongs with the firmware |
| `tools/log-triage` | candidate: own repository | host tool on homenodellm, not in any image |
| `tools/assistant` | later, if it becomes its own product | today it is an experiment in the image |

The rule for later: a part gets its own repository when it has its own
release cycle, its own toolchain, or users outside this project. Being
large is not enough.

## Migration

Each step is one small PR, and the scripts work after each one. The steps
that touch `test-plan.md` are done when no row is **on the Pi**, after the
Pi agent and the coordinator have been told, so open branches can rebase
quickly.

1. **This RFC.** The maintainer accepts, changes or rejects it.
2. **STATUS.md and `scripts/status.py`.** Remove `docs/status.md` and move
   its findings. Add the 8 KB check. Every brief and agent instruction
   names STATUS.md.
3. **Archive the test reports.** Split `test-reports.md` by month into
   `docs/test-reports/` and leave an index page at the old path. Change
   the help text of `pi-test.py` and `sd-stress.py`, and where
   `update-stable.sh` writes.
4. **Move done rows.** Create `docs/test-done/YYYY-MM.md`. Change
   `pr-order.py`, and the number rule in the rules section.
5. **Test files.** Create `docs/tests/`, a template, and a rule that new
   rows get a file. Move the steps of the open queue rows into files in
   the same PR, one file per row, and shorten the rows. Add a CI warning
   for rows over 400 characters.
6. **Topic status blocks.** One PR per topic owner, or one PR for all.
   Then add the freshness warning to `check-topics.py`.
7. **Tools.** Move `rpi-boot-stub` and `log-triage` only if the
   maintainer wants that. This step is independent of the others.

Steps 2 and 3 can go in parallel. Steps 4 and 5 change the same file, so
they go one after the other.

### Who finds what afterwards

- **A new session:** STATUS.md, then the topic page's status block, then
  the files its brief names.
- **Pi agent:** the queue in `test-plan.md` for the order, then
  `docs/tests/NNNN.md` for the steps. It writes the report into that same
  file and the done row into `docs/test-done/<month>.md`.
- **Coordinator:** runs `scripts/status.py`, updates the topic lines in
  STATUS.md, and watches the queue as today, but it now reads a 10–15 KB
  file instead of 190 KB.
- **Architecture:** STATUS.md and ROADMAP.md for the overview, then the
  topic pages.

## Alternatives considered

- **Queue as GitHub issues or a Project board.** Every row would be
  searchable, and nothing would conflict. But "take the Pi" is a commit
  on main today, which makes it atomic, and every script reads the queue
  offline from git. A board would need the GitHub API and a token on
  homenodellm for every check. Revisit if the conflicts remain after
  step 5.
- **Only archive done rows, as first proposed.** That removes 105 KB of
  190 KB, but leaves the 76 KB queue. Short rows with test files are what
  makes the plan small.
- **Per-topic test-report files.** Many tests belong to more than one
  topic, for example network and usb, and some to none, for example a
  release check. A file per test has no such ambiguity, and topic pages
  link to the files they need.
- **A separate docs repository.** Docs and code would no longer change in
  the same PR, and `check-topics.py` relies on that.
- **Splitting into many repositories.** See [Repositories](#repositories):
  it does not solve the context problem and adds cross-repository PRs.

## Open questions

- Should STATUS.md include the sessions from the app (name, state)? Only
  the coordinator can see them, so the coordinator would have to write
  that part by hand.
- Should old open queue rows get test files now (step 5), or only when
  they are next touched?
- Should `docs/test-done/` stay in this repository? It grows by about
  100 rows a month, but nobody reads it whole.
- Is 8 KB the right limit for STATUS.md, or should it be smaller, for
  example 4 KB, so it is cheap to read on every coordinator tick?
