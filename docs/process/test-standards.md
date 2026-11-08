# Test standards

How redox-rpi tests its work: the test levels, what a queue row and a report
contain, when a result counts as pass, fail or flaky, and how tests with a
person are prepared. Owner: verification and validation (V&V, the Pi test
agent). Changed by pull request; see the
[project management plan](pmp.md) for the roles.

Status: draft, 2026-11-08 ([#264](https://github.com/redox-rpi/redox-rpi/issues/264)).

The day-to-day rules for the one test Pi (queue, serial console, stable
image) stay in [test-plan.md](../test-plan.md). This page says what a test
must be; the test plan says how to get the Pi.

## Test levels

Every change is tested at the cheapest level that can catch its bugs, and
then at each higher level it needs. A level never replaces a higher one that
the change needs: QEMU has no Ethernet chip, no VideoCore and no real SD
timing, so anything that touches those needs the real Pi.

| Level | Where | What it catches | Who runs it | Needed for |
|---|---|---|---|---|
| 1. Host unit test | build host or Mac, `cargo test`, Python scripts | logic errors in tools, parsers, drivers' pure code | the owner | every change that has testable host code |
| 2. QEMU | `scripts/qemu-raspi3b.sh --test`, `scripts/qemu-run.sh` | boot, kernel, relibc, programs that do not need Pi hardware | the owner, before queueing a Pi row | every image change |
| 3. CI | GitHub Actions ([testing.md](../testing.md#ci)) | the same as 1 and 2, on every push and PR, and the regression set | automatic | every PR to main |
| 4. Real Pi, automated | the test Pi over network boot, `scripts/pi-test.py`, devsh, serial console | drivers, timing, network, SD card, firmware, real boot | V&V, in queue order | every feature or fix before its PR is ready; every release candidate; the nightly stable image |
| 5. Person test | the test Pi with a person: keyboard, screen, card swap, cables, listening | what only a person can see, hear or plug in | the maintainer or another person, prepared by V&V | only what levels 1–4 cannot check |

Rules:

- A feature reaches main only after a level 4 pass of its exact build (or
  level 5, if it needs a person).
- An image change that skipped QEMU (level 2) is not queued for the Pi
  unless the row says why QEMU cannot run it.
- Each row and report names its level.

## Queue rows

A row in [test-plan.md](../test-plan.md#queue) contains the following.
V&V may send back a row that lacks any of them.

1. **Number.** Use the next free number, as in rule 1 of the test plan.
2. **Purpose and reference.** Name the requirement (`REQ-…`, once
   requirement IDs exist) or the issue the row verifies. Use the PR too, if
   there is one. Say in one sentence what question the row answers.
3. **Owner.** The session or person who asked for it, and who gets the
   result.
4. **Image.** The exact build ID for each image, where it is on the build
   host, and its manifest. A row that names only a branch is not ready.
5. **Steps.** Number them and write the exact commands. For each step, say
   where it runs: devsh, serial or host. Use only commands that exist on
   Redox (for example, Redox `grep` has no `-e` and ion needs `&>`).
6. **Pass criteria.** Say what counts as pass for each step, with numbers
   where there are numbers: "0 of 300 starts failed", "19 of 20 or better",
   "median of 6 rounds". Write "observe only" for a step that just records
   a value.
7. **Expected known failures.** Name the checks that are expected to fail
   and their issues, so they do not hide a new failure.
8. **Needs.** For example: network boot, serial, a person, a card swap, a
   power cycle or the screen.
9. **Duration.** About how long it takes, so the queue can be planned.
10. **Status.** One of `waiting`, `ready`, **on the Pi**, `paused` or
    `replaced by #N`.

## Reports

A report goes into [test-reports.md](../test-reports.md), with one line in
"Done" in the test plan. It contains:

1. **Title.** The row number, the issue or requirement, the PR, a short
   name, the date and who tested.
2. **Builds.** The build ID of every image that ran, read from the running
   Pi (`/etc/redox-rpi-release`), not only from the row.
3. **Setup.** The level, how it booted (network or card), the hardware
   attached, and any change from the row's steps, with the reason.
4. **Result first.** Start with **pass**, **fail**, **flaky** or **open**,
   then one sentence on what it means for the issue.
5. **Per step.** The measured values and outputs, enough that someone who
   was not there can check the verdict. Long output stays on the build host;
   the report says where (`~/pitest-tools/tNNN/`).
6. **Unexpected events.** For example: a panic, a hang, a boot without
   network or a needed power cycle. Each one gets a link to its issue, or a
   new issue.
7. **Restore.** The stable build that was restored, and whether it answered
   afterwards.

The issue gets a short comment in plain English: the result, the numbers
that matter, and a link to the report. The owner gets the same by message.

## Verdicts

| Verdict | When |
|---|---|
| **pass** | every step meets its pass criteria; no panic, hang or other new failure |
| **fail** | a step misses its criteria and the failure reproduces (see the rerun rule), or there is a panic or hang that the change could have caused |
| **flaky** | a step fails in some runs and passes in others on the same build |
| **open** | a step could not be run (person missing, tool missing, setup failure); the report says which and what is needed |

A known failure listed in the row (with its issue) does not turn a pass into
a fail. A failure that is not listed does, even if it looks unrelated, until
it is shown to happen on stable too.

### Rerun rule

- **Rerun.** After an unexpected failure, run the same step on the same
  build again, up to two more times, after a fresh boot.
- **Fail.** If the failure comes back, the verdict is fail.
- **Flaky.** If it does not come back, the verdict for that step is flaky.
  The report gives the rate, for example "1 of 3 boots". Every flaky result
  gets an issue, or a comment on the existing one (for example #28, #92 or
  #217).
- **Flaky is not pass.** A flaky step counts as pass only if the row's
  criteria allow a rate ("19 of 20 or better") and the measured rate meets
  it.
- **Compare with stable.** If a failure may not come from the change, run
  the same step on the stable image. If stable fails the same way, the
  failure is not the change's. The report says so and links the stable issue.
- **Retries are not reruns.** A retry inside a tool (for example bench.py's
  "nc did not connect, attempt 2") is not a rerun. The report counts such
  retries.

## Reproducible builds

- **Clean builds only.** A result that a merge or a release relies on comes
  only from a clean build. That means no `--allow-dirty` and no
  `-dirty` in the build ID, built by `scripts/build-image.sh` from a pushed
  commit, with its manifest.
- **Dirty builds are for diagnosis only.** A dirty build may be used to
  find a bug or to try a fix. Its report says "exploratory, dirty build"
  in the result, and the PR still needs a clean pass before it is ready.
- **Rebuild before the final run.** When a fix lands on the branch, the
  owner rebuilds clean and queues the final run with the new build ID.
- **Releases.** Release candidates follow
  [release-process.md](../release-process.md).

## Benchmarks

Comparisons between images (speed, boot time, memory) follow these rules,
so a difference is not just noise:

- **Same Pi, same setup.** Use the same network boot and the same cables.
  Record the clocks before and after (`bench.py` prints them) and check
  `throttled`.
- **Alternate the images**, for example A, B, B, A or A, B, C, C, B, A, so
  that drift over time shows up and averages out.
- **At least 6 rounds per image, over at least 2 boots.** Report the median
  and the range.
- **When a difference is real:** call it real only when it is larger than
  the ranges' overlap. Card-bound workloads (SD writes) are noisy; say so
  instead of reading a winner into them.
- **Keep the raw data.** Keep the JSONL or logs on the build host and give
  the path in the report.

## Special tests

V&V owns these tests ([#266](https://github.com/redox-rpi/redox-rpi/issues/266)).
Each one runs at level 4 unless it says otherwise.

| Test | Tool | Typical criteria |
|---|---|---|
| Starts | `pi-test.py` `starts` check, 300 starts; 1000 for a release | 0 failed |
| Reboot loop | reset over devsh or serial, N boots | every boot reaches `redox login:` and devsh; boots without network are counted (#92, #217) |
| SD stress | `scripts/sd-stress.py` | the data reads back with the same hash; no I/O error |
| Soak | the stable image left running with periodic checks | no hang or panic over the run; memory used does not grow |
| Chaos | experiment `chaos` and `scripts/chaos-host.py` ([README](../../patches/experiments/chaos/README.md)) | the system still answers after each fault; no `UNKILLABLE` child |
| Boot timing | `patches/experiments/boot-time/boot-timeline.py` on the serial log | the time to `redox login:`; the 30 s and 35 s groups are known |
| Serial input | `patches/experiments/serial-rx/serial-input-test.sh` | the rate the row names (#28) |
| Listening | a person at the jack (level 5) | the person's own words, recorded as they said them |
| Visual | a person, the NanoKVM, or a screenshot tool (`vc4shot` in vc4d images) | what the row's "screen should show" describes |

## Regression set

A fast set that catches known breakage before a row reaches the Pi
([#265](https://github.com/redox-rpi/redox-rpi/issues/265)). Each past
regression gets a check at the cheapest level that can see it. Examples:

- Audio at half rate (#279). This needs a check of the PWM range and rate
  in QEMU, or in the Pi smoke run.
- An image that boots without devshd, or with devshd not listening.
- A broken drift check (`scripts/check-upstream-drift.py`).

The CI part goes to Release and CI. The Pi part becomes a short smoke run:
`pi-test.py` plus a few area checks, under 10 minutes. Every Pi row starts
with it. This section will list the set once it exists.

## Person tests

A person's time at the Pi is the scarcest resource. Before the person
comes:

1. **Build everything.** Every image the visit needs is built and named in
   the row, with its build ID. SD card images are in `/srv/tftp/sdcard/`
   (http://homenodellm:8080/sdcard/) or on the image server, with the
   command to flash them.
2. **Run the automated parts first.** Every step that does not need the
   person has already run and passed at level 4.
3. **Write the steps for the person.** Use plain, numbered steps: what to
   plug in, what to press, what the screen should show, and what to report
   back. Every key is spelled out.
4. **Bundle the visit.** Put the rows for one visit into as few boots and
   card swaps as possible. The project lead tells the maintainer in
   advance, in one sentence.
5. **Stage the images.** V&V stages the first image and checks it boots,
   just before the visit.
6. **Record the person's words.** The report records the person's
   observations as they said them, marked as theirs.

## Traceability

Each row and report names what it verifies: the requirement (`REQ-…`) once
requirements exist, otherwise the issue and the PR. The trace is: the
requirement, then the issue, the PR, the Pi row, the report, and the release
notes ([pmp.md](pmp.md#traceability)). Quality audits the chain and the test
matrix, which shows which area is tested at which level
([#267](https://github.com/redox-rpi/redox-rpi/issues/267)).
