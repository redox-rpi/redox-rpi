# Quality

Process checks, bug analysis, KPIs and the dashboard, as set out in the
quality management plan [qmp.md](../process/qmp.md). Quality does not
write or run tests: that is V&V (the Pi test agent) and Release & CI.
It audits what is tested and states the gaps as requirements on V&V.
Root-causing a bug stays with its topic owner. Owner: the Quality agent
(since 2026-11-08). KPIs: [kpis.md](../process/kpis.md), from
`scripts/quality-kpis.py`.

## Status (2026-11-10)

- **Merges not built together:** 44 PRs in 30 days were merged on CI
  older than an earlier merge in the same patch directory; one left main
  red for 10 h on 2026-11-07 ([#257](https://github.com/redox-rpi/redox-rpi/issues/257), open).
- **#92 caught once more:** row 316 (2026-11-10) stalled the network
  under `dd` load with vc4d, and `netstack-watch` logged "main loop stuck
  in read"; rows 319 and 321 did not reproduce it.
- Issue types are in use (99 of 109 open issues), #28 and #108 are
  reopened, #92 and #129 have topics. 30 open bugs, no severity yet.
- Nightly 91 % (10 of 11), Pi rows 58 % pass. KPIs: [kpis.md](../process/kpis.md).

## Coverage audit

Where each feature is checked today (audit, 2026-11-08). **CI**: GitHub Actions on every PR and push
to main. **Nightly**: `scripts/update-stable.sh` builds main and runs
`scripts/pi-test.py` on the Pi. **Release**: the tests in
[v0.4.0-checklist.md](https://github.com/redox-rpi/redox-rpi/blob/release-v0.4.0/docs/releases/v0.4.0-checklist.md#tests)
(number = test there), run once per candidate. **Person**: needs someone at
the Pi. "–" means nothing.

| Feature | CI | Nightly | Release | Person |
|---|---|---|---|---|
| Patches apply (pinned / newest upstream) | `check-patches.sh` / weekly | – | – | – |
| Kernel boots, 4 CPUs, login | QEMU, upstream daily userspace | yes | 1 | – |
| Our full image boots to login | – | Pi, network boot | 1, 5 | 6 (SD) |
| Program starts (ld.so, relibc) | – | 300 starts | 1 (1000) | – |
| `ps`, terminal output | – | yes | 1 | – |
| SD card read/write `/scheme/data` | – | one file | 18 (`sd-stress.py`) | 6 |
| Root on the SD card, U-Boot menu | – | – | – | 6 |
| Clean boot log (no `pcid`/`rtcd`, daemons start) | – | – | 3 | – |
| LAN7800 DHCP, devsh | – | implied by the test | 1 | – |
| Network stays up over time (#92, #217) | – | – | – | – |
| Long TCP download | – | – | 20 (`nc`) | – |
| Several adapters, RTL8153 | – | – | 19 | 9 |
| NTP clock (timesync) | – | – | 4 | – |
| CPU clock 1400 MHz, governor | – | – | 2, 13 | – |
| `reboot` | – | – | 5 | – |
| `poweroff` (#7) | – | – | – | – |
| Serial console input (#28) | – | – | 11 | – |
| USB keyboard, hotplug, mouse | – | – | – | 7, 8, 21 |
| HDMI console | – | – | – | 7 |
| Desktop, rc2 beta (PR #255) | – | – | – | 22 (planned) |
| PWM audio, beta | – | – | 16 | 17 |
| PWM audio in QEMU (`qemu-check.sh`) | – | – | by hand | – |
| GPIO/I2C/SPI without wiring | – | – | 14 | wired: rows 38–40 |
| relibc signal registers (`sigregs`) | Linux, weekly | – | – | – |
| relibc test suite on Redox (#13) | – | – | 15 (QEMU, by hand) | – |
| Zero 2 W | – | – | – | 9 |
| Release download, checksum | – | – | 10 | – |
| Boot time (#47) | – | – | – | – (one-off rows) |
| `pkg install` (#197) | – | – | – | – |
| Fault injection (#196) | – | – | – | – (one-off rows) |

## Known issues

Recurring or flaky bugs: classification, pattern, root-cause status and
owner, as in [qmp.md](../process/qmp.md#bug-analysis). The topic owner
finds the cause; Quality tracks the pattern and the recurrence. Rows that
name each bug (sightings and repro attempts): [kpis.md](../process/kpis.md#rows-that-name-a-tracked-bug).

| Issue | Status | Last seen | Pattern | Check (V&V) |
|---|---|---|---|---|
| [#92](https://github.com/redox-rpi/redox-rpi/issues/92) network lost minutes after boot | open Bug, network | row 316, 2026-11-10: under 4× `dd bs=16M` with vc4d, `netstack-watch: main loop stuck in read`; 319, 321 (same load, with and without vc4d) no stall | rows 211, 213, 226, 229, 237 B, 285, 316; netstack, dwc2d, devshd alive, no ARP answer; first time a watch caught the place (read loop) | none in the nightly: [#259](https://github.com/redox-rpi/redox-rpi/issues/259), V&V special tests #266 |
| [#217](https://github.com/redox-rpi/redox-rpi/issues/217) login without network, devshd never listens | open Bug, network | row 276, 2026-11-07 (no devsh for 3 min, then a power cycle) | rare boot race; 0 in 39 warm resets (row 266); 1 in 1000 boots on stable 643 (V&V count, rows 276, 285) | V&V special tests #266 |
| [#28](https://github.com/redox-rpi/redox-rpi/issues/28) serial console drops characters | **reopened** Bug, stability | rc2 row 292: 19 of 20 | 18–20 of 20 in every release since v0.3.0; within the limit V&V set (19 of 20) | release 11 |
| [#108](https://github.com/redox-rpi/redox-rpi/issues/108) `pthread_exit` of the last thread ends with SIGKILL | **reopened** Bug, stability; fix only in experiment `self-exit-status` | rc1 test 15 (`pthread/exit` NEW) | in every release image | release 15 |
| [#79](https://github.com/redox-rpi/redox-rpi/issues/79) netstack finds no adapter at boot | open, usb/boot | – | `/scheme` listing loses names | none |
| [#129](https://github.com/redox-rpi/redox-rpi/issues/129) devshd stops accepting after a long run | open Bug, network | – | after memory-heavy runs | none |
| [#119](https://github.com/redox-rpi/redox-rpi/issues/119), [#140](https://github.com/redox-rpi/redox-rpi/issues/140), [#141](https://github.com/redox-rpi/redox-rpi/issues/141) long TCP download slows, ENOTCONN | open, network | – | long downloads | release 20 (100 MiB) |
| [#151](https://github.com/redox-rpi/redox-rpi/issues/151) Pi hangs, long SD write during a download | open, usb | row 141 | not in QEMU | none |
| [#205](https://github.com/redox-rpi/redox-rpi/issues/205) `kill()` never reaches a loop on a pipe | open, stability | row 237 | | none |
| [#207](https://github.com/redox-rpi/redox-rpi/issues/207) chaos test froze the Pi ("Lacks grant") | open, stability | row 237 | | chaos rows |
| [#55](https://github.com/redox-rpi/redox-rpi/issues/55), [#86](https://github.com/redox-rpi/redox-rpi/issues/86) Ctrl-C lost or late | open, stability | – | | none |
| [#197](https://github.com/redox-rpi/redox-rpi/issues/197) `pkg` fills the network-boot root | open, boot | – | after 7 rounds | none |
| Half-rate PWM audio (row 279, #250) | fixed in PR #251 (row 283: 44 111 Hz) | 2026-11-07 | rate halved, DMA and tone check looked fine | release 16 only |
| Patch fuzz breaks the drift check (base `0083` after `0041`) | fixed in PR #254 | rc1 build, 2026-11-07 | #81 merged on CI from before #194 landed; main red 10 h | CI on main, unwatched |

## Requirements on V&V

The gaps of the coverage audit, as requirements. V&V (and Release & CI
for CI and the nightly) decide how to test them; Quality checks that they
are met.

Taken on 2026-11-08 (V&V review on PR #262): R2 and R5 into the V&V
regression set (#265), R3 into the special tests (#266); R1, R4 and R10
to Release & CI through the lead; R8 is owner work.

| # | Requirement | Gap it closes | Issue |
|---|---|---|---|
| R1 | A red `QEMU boot test` on main is reported within an hour; a PR is merged only on CI that includes the previous merges in its patch directories; a candidate comes from a green commit | main red 10 h, 43 stale-CI merges | [#257](https://github.com/redox-rpi/redox-rpi/issues/257) |
| R2 | Every nightly checks audio rate (half rate fails), NTP time, CPU clock, a TCP transfer with checksum and one reboot | half-rate audio (row 279) passed `pi-test.py` | [#258](https://github.com/redox-rpi/redox-rpi/issues/258) |
| R3 | Network loss (#92) and boots without network (#217) are counted with a rate, not found by chance | 17 sightings, all by chance | [#259](https://github.com/redox-rpi/redox-rpi/issues/259) |
| R4 | A PR that changes default patches boots our full image (not upstream's userspace) before merge | base and relibc first run on the Pi | [#260](https://github.com/redox-rpi/redox-rpi/issues/260) |
| R5 | rc2: the desktop has a test without a person; console users see no regression against rc1; audio has a tolerance; known issues are in the notes | rc2 checklist | [#261](https://github.com/redox-rpi/redox-rpi/issues/261) |
| R6 | The release checklist records a result per test, so completion can be measured | KPI "checklist completion" | – |
| R7 | Every release feature has an automated check or a named person test (v0.4.0: 4 of 12 have none) | feature coverage 67 % | [#261](https://github.com/redox-rpi/redox-rpi/issues/261) |
| R8 | vc4d's logic builds and tests on a host | 6 tests never run | [#271](https://github.com/redox-rpi/redox-rpi/issues/271) |
| R9 | Our drivers and kernel/relibc patches get clippy for the Redox target, report-only | 198 `unsafe`, no analysis | [#272](https://github.com/redox-rpi/redox-rpi/issues/272) |
| R10 | A CI job "Code quality": pyflakes and `cargo test` gating, the rest report-only | baseline only by hand | [#273](https://github.com/redox-rpi/redox-rpi/issues/273) |

## Next

- Test strategy matrix ([qmp.md](../process/qmp.md#test-strategy)) with
  V&V (#267); coverage kinds and targets
  ([qmp.md](../process/qmp.md#coverage)); code quality baseline
  ([code-quality-baseline.md](../process/code-quality-baseline.md)).
- Weekly KPI report to the lead; where `quality-kpis.py` runs and
  publishes (nightly or the image server): with Release & CI.
- Severity labels `severity:S1`–`S3` (proposal in the QMP, lead decides);
  topic labels for the 8 unlabelled issues (owners).
- #28 and #108: reopen or list as known in the v0.4.0 notes (Release & CI).
- `docs/known-issues.md` still describes v0.2.0 (Release & CI).

## Later

- Test coverage per requirement, once the PMP has requirements.
