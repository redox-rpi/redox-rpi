# v1.0 criteria: issues, trace and gaps

Status: **draft**, 2026-11-11. Owner: the Requirements agent.

The 24 criteria of [RFC 0003](../rfcs/0003-v1-criteria.md) (draft) mapped to
their Feature issues, the issues that implement them and the tests that
verify them. Rules: [RMP](../process/rmp.md). Until the maintainer decides
RFC 0003, every criterion is **proposed**: a Feature issue with
`Status: proposed` and no milestone.

## Criteria and Feature issues

"to create" means the Feature issue does not exist yet. Existing issues get
the requirement block on top and the ID in the title (RMP, Storage).

| ID | Name | Prio | Theme | Feature issue | Implemented by | Verified by |
|---|---|---|---|---|---|---|
| REQ-BOARD-2 | Supported boards for 1.0 | must | Towards 1.0 | to create | #8 (closed) | v0.4.0 rows 292, 286; **gap**: no automated row on the Zero 2 W |
| REQ-BOARD-3 | Other BCM2837 boards | should | Towards 1.0 | to create | **none** (Pi 3B needs a LAN9514 driver) | **gap** |
| REQ-BOARD-4 | Raspberry Pi 400 | must | More boards | #350 | #299 (WP 3.2), #300–#307, #332, PR #310 | no row yet |
| REQ-STAB-4 | No open crash bugs | must | Towards 1.0 | to create | crash bugs #207, #92, #217, #151, #205, #4, #129 | inspection, not done |
| REQ-STAB-5 | Long run | must | Towards 1.0 | to create | #259 | **gap** |
| REQ-STAB-6 | Chaos test | must | Towards 1.0 | #196 | #196, #207 | row 237 (dev build); not on a candidate |
| REQ-STAB-7 | Power loss | must | Devices | to create | **none** | **gap** (needs the Shelly plug rules) |
| REQ-STAB-8 | Reboot cycles | must | Towards 1.0 | to create | #217 | rows 264, 266 (dev builds); **gap** for 500 reboots, 50 cold starts |
| REQ-FW-5 | Power off | should | Devices | #7 | #7 | **gap** |
| REQ-NET-6 | SSH server | must | Remote access | #45 | #45, #190, #184; experiment `sshd` | no row on a candidate |
| REQ-SEC-1 | No default password on the network | must | Remote access | to create | **none** | **gap** |
| REQ-SEC-2 | Only wanted services listen | must | Remote access | to create | **none** | **gap** (no port-scan row) |
| REQ-SEC-3 | Signed packages and images | must | Devices, Towards 1.0 | to create | #44, #237 | **gap** |
| REQ-SEC-4 | Security fixes | must | Towards 1.0 | to create | #190 | **gap** (no advisory check per release) |
| REQ-PKG-1 | Install and remove packages | must | Devices | #44 | #44, #197 | **gap** on a candidate |
| REQ-PKG-2 | Base set of programs | must | Remote access | #40 | #40 | **gap** |
| REQ-PKG-3 | System update without a new card | must | Towards 1.0 | to create | **none** (needs an Architect RFC) | **gap** |
| REQ-DOC-1 | Beginner guide | must | Towards 1.0 | to create | **none** (user-guide.md, getting-started.md exist) | **gap** (no newcomer test) |
| REQ-DOC-2 | Hardware list | should | Towards 1.0 | to create | **none** | **gap** |
| REQ-REL-6 | All must requirements verified | must | Towards 1.0 | to create | per-release trace ([trace-v0.4.0.md](trace-v0.4.0.md)) | inspection per release |
| REQ-REL-7 | Nightly green before the release | must | Towards 1.0 | to create | #258, nightly reports (redox-rpi/pi-tests) | **gap** (no streak count) |
| REQ-REL-8 | Support promise | must | Towards 1.0 | to create | **none** | **gap** |
| REQ-REL-9 | Close to upstream | should | Towards 1.0 | to create | weekly upstream drift report | inspection, not done for a release |
| REQ-CON-3 | Keyboard layouts | must | Devices, More boards | #36 | #36, #305 | **gap** |

## Sub-issues to link

Implementing issues without a parent, to hang under their Feature issue
once it exists:

| Issue | Parent |
|---|---|
| #299 (WP 3.2 Pi 400) | #350 REQ-BOARD-4 |
| #190 (OpenSSH update) | #45 REQ-NET-6 |
| #197 (root fills up) | #44 REQ-PKG-1 |
| #259 (nightly soak) | REQ-STAB-5 |
| #258 (nightly checks) | REQ-REL-7 |

## Gaps

**Themes with no requirement** (roadmap #316):

- **Network**: no 1.0 criterion. Wi-Fi is open question 2; without it the
  Zero 2 W has no network unless a HAT is fitted.
- **Smart home**: none by design (features, not 1.0 criteria).
- Desktop and audio stay beta unless open question 3 says otherwise.

**Criteria with no implementing issue** (8): REQ-BOARD-3, REQ-STAB-7,
REQ-SEC-1, REQ-SEC-2, REQ-PKG-3, REQ-DOC-1, REQ-DOC-2, REQ-REL-8. These
need work packages (RFC 0003, Steps 3); proposals in
[v1-work-packages.md](v1-work-packages.md).

**Criteria with no test on a release candidate** (20 of 24). Partly
covered: REQ-BOARD-2 (rows 292, 286), REQ-STAB-6 (row 237, dev build),
REQ-STAB-8 (rows 264, 266, dev builds), REQ-REL-6 (trace inspection).
V&V has to plan the long-run, chaos, power-loss, reboot, port-scan and
update rows (RFC 0003, Steps 4); drafts in
[v1-test-drafts.md](v1-test-drafts.md).

**Other**

- REQ-STAB-4 counts "crash" bugs, but there is no `crash` label. Quality's
  `severity:S1` (QMP) may serve instead (open question 8).
- The areas `PKG`, `SEC` and `DOC` enter the RMP when RFC 0003 is accepted.
- Pi 400 criteria (REQ-BOARD-4, REQ-CON-3) have no milestone until the
  board arrives (open question 10).
