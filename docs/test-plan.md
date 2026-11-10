# Test plan for the real Pi

**The queue and the reports have moved** to the repository
[redox-rpi/pi-tests](https://github.com/redox-rpi/pi-tests) (RFC 0002
phase 2, maintainer's decision 2026-11-09). Add and change rows only there.

- **Queue:** [pi-tests queue.md](https://github.com/redox-rpi/pi-tests/blob/main/queue.md),
  with the rules for taking and returning the Pi. Only the row marked
  **on the Pi** may stage files on the TFTP server, reboot the Pi or run
  tests on it.
- **Reports:** [pi-tests reports/](https://github.com/redox-rpi/pi-tests/tree/main/reports),
  one file per row (`reports/<row>.md`).
- **Hardware, serial console, stable image, nightly run:**
  [pi-tests lab.md](https://github.com/redox-rpi/pi-tests/blob/main/lab.md).
- **Release evidence:** [pi-tests releases/](https://github.com/redox-rpi/pi-tests/tree/main/releases).

The test code stays here: [`scripts/pi-test.py`](../scripts/pi-test.py), the
other scripts, `tools/` and [testing.md](testing.md). What a row and a
report contain: [process/test-standards.md](process/test-standards.md).
The rows and reports up to the switch-over are in this file's history and
in pi-tests.

## Queue

Moved to [pi-tests queue.md](https://github.com/redox-rpi/pi-tests/blob/main/queue.md).

## Serial console

Moved to [pi-tests lab.md](https://github.com/redox-rpi/pi-tests/blob/main/lab.md#serial-console).

## Stable image

Moved to [pi-tests lab.md](https://github.com/redox-rpi/pi-tests/blob/main/lab.md#stable-image).
Restore it after a test with `cp -p /srv/tftp/stable/* /srv/tftp/redox/`.

### Nightly update

Moved to [pi-tests lab.md](https://github.com/redox-rpi/pi-tests/blob/main/lab.md#nightly-update).

### Nightly visit images

Moved to [pi-tests lab.md](https://github.com/redox-rpi/pi-tests/blob/main/lab.md#nightly-visit-images).

## Building on the build host

Moved to [testing.md](testing.md#building-on-the-build-host).

## Merging through pull requests

Since 2026-10-30 every change that goes to main through a branch goes
through a pull request (maintainer's decision):

1. **Owner session:** opens a draft pull request from its branch and works
   on it until CI is green, the Pi test passed (link to its report in
   `docs/test-reports.md`) and the topic page is up to date. The
   description says in plain English what changes, which issues it closes
   (`Fixes #N`) and what still needs a person.
2. **Coordinator:** checks green CI, the passed Pi test, no conflicts with
   main, the next free patch numbers and no overlap with other open pull
   requests. If all is fine it marks the pull request ready for review and
   tells the maintainer; if not, it comments in the pull request and sends
   it back to the owner.
3. **Maintainer:** merges it in GitHub. The merge is the approval; nobody
   else merges a pull request.

Without a pull request, straight to main: the queue and reports
(`docs/test-plan.md`, `docs/test-reports.md`), topic pages and other
docs-only or script fixes.

## Worktrees on the Mac

Rule since 2026-11-07 (maintainer's decision), so `~/Developement` stays tidy:

- The main checkout stays at `~/Developement/RedoxOS`. Don't move it: the
  sessions and their memory are tied to that path.
- Extra worktrees go only under `~/Developement/redox-wt/<name>` (or the
  app's own `.claude/worktrees/` or a session scratchpad), never directly
  in `~/Developement`.
- Push the branch before you leave a worktree. The session that made a
  worktree removes it (`git worktree remove`) once its pull request is
  merged or closed.
- The coordinator checks about once a week and removes clean worktrees
  whose branch is merged or pushed and that no running session uses.
