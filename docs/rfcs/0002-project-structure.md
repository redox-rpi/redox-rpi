# RFC 0002: Project structure: clean up, split the repositories, fit a session's context

| | |
|---|---|
| Status | Draft (second version, 2026-11-08) |
| Created | 2026-11-08 |
| Issue | – |
| Releases | none; repositories, documentation, scripts and GitHub setup |

## Summary

The project has outgrown its single repository and its single issue list.
No session can see the whole state of the project any more. On 2026-11-07
the architecture session did not know it. The maintainer's direction:

> "Es wird zu groß alles in eine / wenigen repositories zu halten."

> "… sollten wir die Struktur aufräumen und auch Issues und Bugs in dem
> jeweiligen Repository managen. Darum prio 1 ist aufräumen, Prio 2 ist es
> etwas professioneller aufzusetzen."

This RFC proposes, in this order:

1. **Clean up** what is stale today: branches, pull requests, experiments,
   issues, worktrees, and topics that have no owner.
2. **Fit a session's context:**
   - a short STATUS.md as the single entry point,
   - a test queue with short rows,
   - one file per Pi test,
   - a status block at the top of each topic page.
3. **Split the repositories.** Our own drivers, tools, the assistant, the
   host tools and the test records each get a repository of their own, with
   their own issues. `redox-rpi` keeps the build, the patches against
   upstream, the image configuration and the release. A lock file pins
   each repository by commit, so one command still builds a reproducible
   image.
4. **Set GitHub up properly:**
   - every bug and task lives in the repository of its code,
   - the issue types Bug, Feature and Task,
   - features as issues in `redox-rpi`, with sub-issues in the component
     repositories,
   - one GitHub Project across the organization,
   - one set of labels for every repository.

Steps 1 to 3 are the maintainer's priority 1, step 4 is priority 2.

## Motivation

### What a session reads today

Measured on main on 2026-11-08, at about 4 bytes per token:

| File | Size | Tokens |
|---|---|---|
| `docs/test-reports.md` | 610 KB | ~150k |
| `docs/test-plan.md` | 190 KB | ~47k |
| ... of that, the open queue (38 rows) | 76 KB | 19k |
| ... of that, done rows (about 260) | 105 KB | 26k |
| `docs/status.md`, last updated 2026-10-24 | 15 KB | 4k |
| all topic pages | ~210 KB | ~52k |

The two test documents alone are bigger than one context window. Even the
open queue is 76 KB, because each row carries its full steps.

### One repository for everything

- **Our own drivers live as patches.** Eight drivers that we wrote
  ourselves are whole new crates inside upstream `base`:
  - `dwc2d`
  - `rpi-firmwared`
  - `bcm2835-sdhostd`
  - `bcm2835-gpiod`
  - `bcm2835-i2cd`
  - `bcm2835-spid`
  - `bcm2835-pwmaudiod`
  - `testnetd`

  The patches that create and change them make up most of the 720 KB in
  `patches/base/`. `dwc2d` alone is spread over about 24 patch files. A
  change to a driver is a diff of a diff. Clippy, tests and code review
  only see the driver once all patches are applied. The driver's history is
  in patch file names, not in git.
- **One issue list for everything.** There are 82 open issues across 18
  `topic:` labels, and none of them has an issue type. A bug in `dwc2d`,
  a feature wish and a CI task sit in the same list. The maintainer wants
  features managed as issues; one flat list does not carry that.
- **History.** 363 of the 589 commits on main since 2026-11-01 touch
  `docs/test-plan.md`. The code history drowns in test rows, and the test
  plan is the file most PRs conflict on.

### Stale state

| What | Count on 2026-11-08 |
|---|---|
| Branches on `redox-rpi` | 251 |
| ... with an open PR | 58 |
| ... whose PR was merged, branch never deleted | 84 |
| ... whose PR was closed without merge | 4 |
| ... that never had a PR | 105 |
| Open PRs | 57, of them 31 opened before 2026-11-01 |
| "Delete head branches on merge" setting | off |
| Experiment directories in `patches/experiments/` | 69 |
| Open issues without a `topic:` label | 8 |
| Open issues with an issue type | 0 of 82 |
| Topics with an owner session (PMP, PR #263) | 7 of about 20 |

## Goals

- Each repository has one purpose, its own issues and its own CI.
- One command builds an image, and the build ID says exactly which commit
  of every repository went in.
- A new session learns the state of the whole project from one file of at
  most 8 KB.
- A session that works on one component reads that component's
  repository, its issues and STATUS.md. It needs nothing else.
- Every script and CI job keeps working after each step.

## Non-goals

- No change to how upstream Redox components are patched. Changes to code
  that upstream owns stay patch files in `redox-rpi`.
- No new build system. The Redox cookbook builds every repository as a
  recipe, as it builds `tools/` today.
- No rewrite of old reports or old history. They are archived as they are.

## Design

### 1. Cleanup first

Cleanup changes no structure. It removes what nobody uses, so the split
moves only what is alive.

| Item | What to do |
|---|---|
| 84 branches with a merged PR | delete; GitHub can restore a deleted branch from its PR |
| 4 branches with a closed PR, 105 without a PR | the owner keeps it, or it becomes a tag `archive/<branch>` and the branch is deleted. Branches that docs link to, such as `tree/t1-sigreturn/…` in old test rows, always get the tag |
| "Delete head branches on merge" | turn on, so merged branches disappear by themselves. This is a repository setting: maintainer's OK |
| 57 open PRs | each owner: rebase and mark ready, or close with a one-line reason. PRs older than 14 days without activity are closed, and the branch is kept as a tag |
| 69 experiment directories | Experiments that became default patches, and experiments that failed, are deleted. They get one line each in an index table in `patches/experiments/README.md`: name, result, last commit. Only live experiments stay |
| Worktrees and clones | Mac: `~/Developement/redox-wt/`, weekly cleanup as today. homenodellm: the sessions' own clones and their `keep/` images, deleted once the owner confirms |
| Issues | Close duplicates and issues that a merged PR already fixed. The 8 issues without a topic get one. Retire `docs/tasks.md`, the old T-number map |
| Stale docs | `docs/status.md` (last updated 2026-10-24) gives way to STATUS.md (section 2) |
| Topics without an owner | Group them under fewer owners, see below |

**Topic owners.** The PMP names owners for Graphics, Boot and console,
Network, Audio, Open firmware, TCC and Log triage. Proposed for the rest:

| Topics | Owner |
|---|---|
| stability, chaos, relibc | a "Core" owner (new topic session) |
| sdcard, dma, firmware interface, gpio, usb | the drivers repository's owner (new) |
| ci | Release and CI |
| zero2w, boards | a "Boards" owner, when that work resumes |
| assistant, smarthome | parked on the wish list, no owner until work resumes |

### 2. Fit a session's context

These parts are the same as in the first version of this RFC. Where the
files live changes with the split (section 3).

**STATUS.md**, in the root of `redox-rpi`, is the single entry point. It
holds at most 8 KB, and CI warns above that. It has two parts:

- **Generated** by `scripts/status.py`, run by the coordinator and by the
  nightly:
  - the newest release, and commits since it,
  - the stable image,
  - the test on the Pi and the next three in the queue,
  - open PRs across the organization, with their labels and CI state,
  - the GitHub Project's counts per release and per status.
- **Hand-written** by the project lead:
  - one line per topic: state, next step, link,
  - at most five decisions that wait for the maintainer.

Every session brief starts with "read STATUS.md".

**The test queue holds only open rows, one short line each**, with the
same columns as today. The steps live in **one file per test,
`tests/NNNN.md`**:

- at the top, the steps, the images and the pass criteria,
- below them, the report, which `pi-test.py --report` appends and the agent
  completes.

Done rows go to `done/YYYY-MM.md`. The old `test-reports.md` is frozen and
split by month into `reports/YYYY-MM.md`.

**Status blocks.** Each topic page and each repository README starts with
at most 10 lines:

- State
- Next
- Open issues
- Tests
- Owner
- the date

The history goes below them.

### 3. Repository split

#### Which repositories

| Repository | Holds | License | Issues there |
|---|---|---|---|
| **`redox-rpi`** (exists) | build scripts, `config/upstream.lock`, the new `config/repos.lock`, patches against upstream code, image configs, experiments, release, product docs (README, user guide, architecture, ROADMAP, STATUS, process docs, RFCs, topic pages) | MIT | **Features** (product level), release planning, build and CI bugs, bugs whose component is not yet known |
| **`drivers`** (new) | our own Pi drivers as one Cargo workspace: `rpi-firmwared`, `bcm2835-sdhostd`, `bcm2835-gpiod`, `bcm2835-i2cd`, `bcm2835-spid`, `bcm2835-pwmaudiod`, `dwc2d`, `testnetd` | MIT | bugs and tasks for these drivers |
| **`vc4d`** (exists) | display and 3D driver | GPL-2.0-only, so it must stay apart from MIT code | its bugs and tasks |
| **`tools`** (new) | the programs we build into the image: `cputop`, `devshd`, `welcome`, `timesync`, `timerd`, `zoneinfo`, `pkg-persist`, `reboot`, `tone`, `i2c`, `spi`, `mcast`, `net6`, `redox-fatfs2`, `socdump`, and the test programs `chaos`, `sigregs`, `polltest`, `icachetest`, `tcc-bench` | MIT | their bugs and tasks |
| **`assistant`** (new) | the local language model assistant | MIT | its bugs, tasks and features |
| **`log-triage`** (new) | host tool on homenodellm, not in any image | MIT | its bugs and tasks |
| **`rpi-open-firmware`** (exists) | open VPU firmware, and `tools/rpi-boot-stub` moves here: both replace parts of the closed boot chain | as today | its bugs and tasks |
| **`relibc`** (exists) | relibc with our patches as branches | MIT | relibc bugs (moved from `topic:relibc`) |
| **`pi-tests`** (new) | the test queue, `tests/NNNN.md`, `done/`, `reports/`, nightly reports | MIT | requests for tests and test-lab problems |
| **`.github`** (exists) | org profile, issue forms for Bug, Feature and Task (GitHub uses them for every repository without its own), the shared label list | | none |

**Why one `drivers` repository and not one per driver.** The drivers share
helpers (`common`, physmap, the firmware mailbox) and one CI. Each would
otherwise need its own lock line, its own labels and its own workflows.
Eight small repositories also make it harder to find things, not easier.
A driver that gets a life of its own, such as `dwc2d` if USB grows, can
move out later with `git filter-repo`.

**Why the test records leave `redox-rpi`.** They change many times a day,
are written mostly by the Pi agent, and need no code review. In their own
repository:

- the code history of `redox-rpi` is readable again,
- PRs stop conflicting on `test-plan.md`,
- the Pi agent can commit its rows without a PR there, if the maintainer
  agrees. "Take the Pi" stays an atomic commit to that repository's main.

**What stays in `redox-rpi`, and why.** The build, the two lock files, the
patches against upstream code, the image configs and the experiments define
together what one image is. They change together, and one PR must be able
to change them together. Small changes to upstream crates, such as a
two-line fix in `netstack` or an init entry that starts one of our
drivers, stay patches. Only whole crates that we wrote leave. Host
scripts that the build uses (`build-image.sh`, `qemu-raspi3b.sh`,
`pi-test.py`, `redoxfs-file`) stay, so that a clone of `redox-rpi` can
still build and test.

#### How the image build pins the other repositories

The new file `config/repos.lock` uses the same format as `upstream.lock`:

```
# repository                                 commit
drivers  https://github.com/redox-rpi/drivers  3f2a…
tools    https://github.com/redox-rpi/tools    91c0…
assistant https://github.com/redox-rpi/assistant 5d7e…
vc4d     https://github.com/redox-rpi/vc4d     0b44…
```

- `scripts/build-image.sh` writes one recipe per crate of each repository,
  with `git = <url>` and `rev = <commit>`. Today it writes recipes with a
  path source for `tools/`. Each crate keeps its `recipe.toml` in its own
  directory, as in `tools/` today.
- The drivers depend on upstream `base` crates (`common`, `daemon`,
  `scheme-utils`) as git dependencies at the `base` commit from
  `upstream.lock`. In the image build, `build-image.sh` adds a `[patch]`
  section that points them at the patched `base` source of the same build.
  It already does this for `fdt`. So the drivers build against exactly the
  `base` the image has, and on their own (`cargo check` in their CI)
  against upstream `base`.
- **Build ID and manifest.** `repos.lock` is a file in `redox-rpi`. A new
  commit of a component reaches the image only by a commit that changes
  the lock, and that commit changes the build ID. The manifest records
  every repository's commit, as it records the upstream commits today.
- **One command:** `scripts/build-image.sh` as today.
- **Working on a component:** `scripts/build-image.sh --local
  drivers=../drivers` builds from a local checkout. The build ID then gets
  a `+local` mark, and like `--allow-dirty` such builds are not reported as
  results.
- **Bumping the lock:** a PR in the component repository is merged first;
  then a one-line PR in `redox-rpi` changes `repos.lock` and runs the
  image tests. `scripts/bump-repo.sh <name>` writes that PR. A Pi test row
  names the lock PR, so the tested image is what gets merged.

#### Moving a driver out of the patches

For each driver:

1. Apply our `base` patches to a clone of `base` at the pinned commit.
2. Run `git filter-repo --subdirectory-filter drivers/<path>`. The
   driver's commits, with their authors and messages, become the history in
   the new repository.
3. In `redox-rpi`, replace the driver's patches with one lock line and,
   where needed, a small patch that starts the driver from init.
4. Build the image with and without the move. The two images must have
   identical driver binaries, apart from paths in debug info. Then run QEMU
   and one Pi row.

Order, from the least to the most entangled: `bcm2835-pwmaudiod`,
`bcm2835-spid`, `bcm2835-i2cd`, `bcm2835-gpiod`, `testnetd`,
`rpi-firmwared`, `bcm2835-sdhostd`, and `dwc2d` last.

### 4. Issues, types, labels and the Project

#### Where an issue lives

- **Bugs and tasks** live in the repository of the code they concern. A
  bug whose component is not known yet is opened in `redox-rpi` and moved
  during triage.
- **Features** live in `redox-rpi`, because a feature is something the
  product does, and it often spans repositories. Example: "sound from the
  headphone jack" needs the driver in `drivers`, the init entry in
  `redox-rpi` and a Pi test in `pi-tests`. The work is done in **sub-issues**
  (GitHub's parent and child issues, which work across repositories of one
  organization), each in the repository of its code.

#### Issue types

These are the organization's three existing types:

| Type | Use | Must contain |
|---|---|---|
| **Feature** | a capability the product should have | what the user can do, acceptance criteria, the requirement ID if there is one, the release |
| **Task** | one piece of work in one repository, ideally one PR | what is done, where, how to check it |
| **Bug** | something does not work as it should | steps, expected and actual result, build ID, log excerpt |

Requirements can become Feature issues, with their ID and acceptance
criteria in the issue. The Requirements agent defines the format. This RFC
only fixes where they live (`redox-rpi`) and that tasks hang below them as
sub-issues. The GitHub labels `bug` and `enhancement` are replaced by the
types and removed.

#### Labels

One list for every repository, kept in `.github/labels.yml` and applied by
`scripts/sync-labels.sh`:

- `needs:pi`, `needs:qemu`, `needs:zero2w`, `needs:nothing`
- `in-progress`, `merge-next`
- `upstream`, `waiting-upstream`
- `good first issue`, `help wanted`, `duplicate`, `wontfix`
- `topic:<name>`, only in `redox-rpi` and `pi-tests`; elsewhere the
  repository is the topic

#### Moving the existing issues

1. Sync the labels into every target repository first.
2. Record each issue's labels.
3. Move the issue with `gh issue transfer <n> redox-rpi/<repo>`, then set
   the labels again. GitHub keeps comments and assignees, but not every
   label and milestone.
4. Set the issue type at the same time. All 82 open issues are touched
   once anyway.
5. Old links keep working: GitHub redirects a moved issue's old URL. A new
   reference across repositories is written `redox-rpi/drivers#12`. The
   coding standards (draft PR #269) adopt this rule.

The first mapping comes from the labels:

| Labels | Target |
|---|---|
| `topic:usb`, `topic:sdcard`, `topic:gpio`, `topic:audio`, and the driver part of `topic:firmware` | `drivers` |
| `topic:relibc` | `relibc` |
| `topic:assistant` | `assistant` |
| `topic:graphics`, display driver part | `vc4d` |
| `topic:verification`, and test-lab issues | `pi-tests` |
| `topic:network` | stays, mostly netstack patches against upstream; `dwc2d` and LAN7800 driver bugs go to `drivers` |
| `topic:ci`, `topic:boot`, `topic:stability`, `topic:time`, `topic:quality`, `topic:zero2w`, `topic:smarthome`, `topic:tcc` | stay in `redox-rpi` |

Each owner checks its own issues before they move.

#### Tracking work across repositories

- **One GitHub Project for the organization**, "redox-rpi", which adds new
  issues and PRs from every repository by itself. Its fields:
  - Status: Todo, In progress, In review, Pi test, Done
  - Release
  - Type
  - Topic
  - Owner: the agent session or person

  This gives one board for the whole project, and `scripts/status.py`
  reads its counts.
- **Milestones stay in `redox-rpi` only**, on Feature issues. They drive
  the release notes. Tasks and bugs in component repositories carry the
  release in the Project's Release field. That way, milestones are not
  copied into every repository.
- The `zehrer-xyz` token has no `project` scope today (`read:org, repo,
  workflow`). The maintainer adds the scope with
  `gh auth refresh -h github.com -s project`, as the `zehrer-xyz` user.
  Sessions do not change token scopes.

### How scripts and CI keep working

| Script / job | Today | After |
|---|---|---|
| `build-image.sh` | `tools/*/recipe.toml` as path recipes | `repos.lock` as git recipes; `--local` for a checkout |
| `pr-order.py` | queue and done rows in `origin/main:docs/test-plan.md` | the same rows from a clone of `pi-tests` (`PITESTS=~/pi-tests`); open PRs across the org |
| `update-stable.sh` | "on the Pi" in the queue, the stable-image block, appends to `test-reports.md` | queue from `pi-tests`; the stable-image block moves to `pi-tests/stable.md`; the nightly report goes to `pi-tests/reports/nightly/<date>.md` |
| `visit-index.py`, `sdcard-index.py`, `build-visit-images.sh`, `publish-sdcard.sh` | queued rows naming ``nightly visit `x` `` | the same text in the short rows, read from `pi-tests` |
| `pi-test.py`, `sd-stress.py` | `--report docs/test-reports.md` in their help | `--report <pi-tests>/tests/NNNN.md` |
| `check-topics.py`, `topics.yml` | `config/topics.map` | rules for moved paths removed; status blocks checked; STATUS.md under 8 KB |
| `check-patches.sh`, `qemu-boot.yml` | patches and our kernel | unchanged; component repositories get their own `cargo check`, clippy and tests |

During the move, both paths work: scripts read `pi-tests` when
`PITESTS` is set, and `docs/test-plan.md` otherwise, until the last
step.

## Migration

Each step is a small PR or a short series of PRs. Scripts work after each
one. Steps that touch the test plan run when no row is **on the Pi**, after
the Pi agent and the coordinator have been told.

**Phase 1: cleanup (priority 1)**

1. This RFC. The maintainer accepts, changes or rejects it.
2. Branches: delete the 84 merged ones; tag and delete the rest the owners
   release; turn on "delete head branches on merge" (maintainer).
3. PRs: owners triage the 57 open PRs.
4. Experiments: index table; delete merged and failed directories.
5. Issues: duplicates, already-fixed, missing topics; retire `docs/tasks.md`.
6. Topic owners as in section 1, written into the PMP.

**Phase 2: context (priority 1)**

7. STATUS.md and `scripts/status.py`; `docs/status.md` removed, its
   findings moved to the topic pages.
8. Create `pi-tests`: move the queue, the stable-image block, done rows by
   month and the frozen reports there. Make the scripts read it
   (`PITESTS`). The steps of the open rows go into `tests/NNNN.md`.
9. Status blocks on the topic pages.

**Phase 3: repository split (priority 1)**

10. `repos.lock`, `--local` and the git recipes in `build-image.sh`, first
    tested with one tool.
11. `tools` repository: move the image tools, with their history through
    `git filter-repo`; move their issues.
12. `log-triage`, `assistant`, and `rpi-boot-stub` into `rpi-open-firmware`,
    each with its issues.
13. `drivers` repository: one driver per PR, in the order from section 3,
    each checked for identical binaries, in QEMU and in one Pi row; move
    the driver's issues with it.
14. Move the remaining issues by the label mapping; set the types.

**Phase 4: GitHub setup (priority 2)**

15. Labels synced to all repositories; `bug` and `enhancement` removed.
16. Issue forms for Bug, Feature and Task in `.github`.
17. The org Project, once the maintainer has added the token scope;
    `status.py` reads it.
18. Features: the Requirements agent writes the first Feature issues for
    v0.5.0, with their tasks as sub-issues.

Phases 1 and 2 can run side by side. Phase 3 needs step 10 first. The
drivers move one at a time, because each move changes `patches/base/`.

### Who finds what afterwards

- **A new session:** STATUS.md, then the README and issues of the
  repository its brief names.
- **Pi agent:** the queue in `pi-tests`, then `tests/NNNN.md`. It writes
  the report into the same file and commits there.
- **Coordinator and project lead:** STATUS.md and the org Project. They
  keep the hand-written part of STATUS.md and the topic owners current.
- **Component owners:** their repository and its issues, plus the Feature
  issues in `redox-rpi` that their tasks hang below.
- **Architect:** STATUS.md, ROADMAP.md, `docs/architecture.md` and the
  RFCs.

## Alternatives considered

- **Everything stays in one repository, only the docs are split** (the
  first version of this RFC). It fixes the context size but not the issue
  list, the driver-as-patches problem, or the history. The maintainer
  asked for more.
- **One repository per driver and per tool.** About 30 repositories, each
  with a lock line, labels and CI. Splitting by purpose (drivers, tools,
  assistant, tests) gives each owner one place without that overhead.
- **Our drivers as a fork of `base`.** It would keep them in the same
  workspace as upstream, but every upstream update would become a merge of
  the whole of `base`. Patches against upstream plus our own crates outside
  it stay smaller.
- **Test queue as a GitHub Project board instead of `pi-tests`.** "Take
  the Pi" would no longer be an atomic commit, and every script would need
  the GitHub API and a token on homenodellm. The Project tracks work, and
  `pi-tests` holds the queue and the records.
- **Milestones in every repository.** GitHub milestones are per
  repository, so they would have to be kept in sync by hand. A Release
  field in one Project does that once.

## Open questions

- May the Pi agent commit to `pi-tests` main without a PR, as it commits
  test rows today?
- Should `dwc2d`, with the USB stack and the LAN7800 path, get its own
  repository right away instead of joining `drivers`?
- Should `relibc` become the place where our relibc patches are made, with
  `patches/relibc/` exported from it, or stay a mirror?
- Should the topic pages of moved components (usb, sdcard, gpio, audio)
  move into the `drivers` repository, with only their status block left in
  `redox-rpi`?
- Is 8 KB the right limit for STATUS.md?
