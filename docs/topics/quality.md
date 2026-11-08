# Quality

What gets tested and how well: test coverage, a fast regression set,
recurring and flaky bugs, release checklists. Running the tests stays with
the Pi test agent and Release & CI; root-causing a bug stays with its topic
owner. Owner: the Quality agent (since 2026-11-08).

## Status (2026-11-08)

- **Main was red 10 hours** (2026-11-07 21:19 to 11-08 07:05): #81 was
  merged on a CI run from before #194 (11-04); together base `0083` needed
  fuzz. rc1 was built from the first red commit; nobody was told.
- **Per-PR CI boots our kernel with upstream's userspace**: base and
  relibc patches first run in the nightly or a Pi row.
- **The nightly checks 6 things**; audio, NTP, reboot and network over
  time only in the release checklist. Half-rate audio (row 279) would
  have passed it. **Network loss (#92, #217)** recurs, nothing counts it.
- Issues: [topic:quality](https://github.com/redox-rpi/redox-rpi/issues?q=is%3Aopen+label%3Atopic%3Aquality).

## Coverage

Where each feature is checked. **CI**: GitHub Actions on every PR and push
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

Recurring or flaky bugs, with where they were last seen. The topic owner
finds the cause; this list tracks the pattern and asks for a test.

| Issue | Status | Last seen | Pattern | Test |
|---|---|---|---|---|
| [#92](https://github.com/redox-rpi/redox-rpi/issues/92) network lost minutes after boot | open, **no topic label** | row 285 try 1, stable 643, 2026-11-07 | rows 211, 213, 226, 229, 237 B, 285; netstack, dwc2d, devshd alive, no ARP answer, nothing on serial; `netstack-watch` rows 263, 267 did not catch it | none: [#259](https://github.com/redox-rpi/redox-rpi/issues/259) |
| [#217](https://github.com/redox-rpi/redox-rpi/issues/217) login without network, devshd never listens | open, network | row 276, 2026-11-07 (no devsh for 3 min, then a power cycle) | rare boot race; 0 in 39 warm resets (row 266) | none |
| [#28](https://github.com/redox-rpi/redox-rpi/issues/28) serial console drops characters | **closed**, still seen | rc1 row 285: 19 of 20, twice | 18–20 of 20 in every release since v0.3.0 | release 11 |
| [#108](https://github.com/redox-rpi/redox-rpi/issues/108) `pthread_exit` of the last thread ends with SIGKILL | **closed**, fix only in experiment `self-exit-status` | rc1 test 15 (`pthread/exit` NEW) | in every release image | release 15 |
| [#79](https://github.com/redox-rpi/redox-rpi/issues/79) netstack finds no adapter at boot | open, usb/boot | – | `/scheme` listing loses names | none |
| [#129](https://github.com/redox-rpi/redox-rpi/issues/129) devshd stops accepting after a long run | open, **no topic label** | – | after memory-heavy runs | none |
| [#119](https://github.com/redox-rpi/redox-rpi/issues/119), [#140](https://github.com/redox-rpi/redox-rpi/issues/140), [#141](https://github.com/redox-rpi/redox-rpi/issues/141) long TCP download slows, ENOTCONN | open, network | – | long downloads | release 20 (100 MiB) |
| [#151](https://github.com/redox-rpi/redox-rpi/issues/151) Pi hangs, long SD write during a download | open, usb | row 141 | not in QEMU | none |
| [#205](https://github.com/redox-rpi/redox-rpi/issues/205) `kill()` never reaches a loop on a pipe | open, stability | row 237 | | none |
| [#207](https://github.com/redox-rpi/redox-rpi/issues/207) chaos test froze the Pi ("Lacks grant") | open, stability | row 237 | | chaos rows |
| [#55](https://github.com/redox-rpi/redox-rpi/issues/55), [#86](https://github.com/redox-rpi/redox-rpi/issues/86) Ctrl-C lost or late | open, stability | – | | none |
| [#197](https://github.com/redox-rpi/redox-rpi/issues/197) `pkg` fills the network-boot root | open, boot | – | after 7 rounds | none |
| Half-rate PWM audio (row 279, #250) | fixed in PR #251 (row 283: 44 111 Hz) | 2026-11-07 | rate halved, DMA and tone check looked fine | release 16 only |
| Patch fuzz breaks the drift check (base `0083` after `0041`) | fixed in PR #254 | rc1 build, 2026-11-07 | #81 merged on CI from before #194 landed; main red 10 h | CI on main, unwatched |

## Fast regression set

Proposal: small checks that run without a person, cheapest first. Each
names the bug class it would have caught. Owners build and run them (CI:
Release & CI; Pi: Pi test agent); Quality writes the check list and reviews
the results.

1. **Every PR, GitHub (minutes, exists):** `check-patches.sh`, QEMU kernel
   boot, topic pages.
2. **Main stays green:** a red `QEMU boot test` on main opens or updates an
   issue, as the nightly does; a PR whose CI ran before main last changed
   is rebased or rerun before it is merged (#81 merged on a 3-day-old
   run); a release candidate is built only from a green commit. Would have
   caught: the 0083 fuzz. ([#257](https://github.com/redox-rpi/redox-rpi/issues/257))
3. **Every PR that changes `patches/`, build host QEMU (~10 min):** the
   full image with `scripts/qemu-raspi3b.sh --data`, then `pi-test.py`
   (100 starts), `patches/experiments/pwm-audio/qemu-check.sh`, and the
   boot log with no panic, no `UNHANDLED EXCEPTION`, no `pcid`/`rtcd`
   line. Would have caught: base and relibc breakage before the nightly.
   ([#260](https://github.com/redox-rpi/redox-rpi/issues/260))
4. **Nightly on the Pi, a few minutes more in `pi-test.py`**
   ([#258](https://github.com/redox-rpi/redox-rpi/issues/258), soak [#259](https://github.com/redox-rpi/redox-rpi/issues/259)):
   - audio: `measured_rate_hz` in `/scheme/audiohw/stats` within 2 % of
     44 100, `tone -s 3 > /scheme/audio` takes 2.7–3.3 s (row 279);
   - clock set by NTP (year ≥ 2026), ARM clock reaches 1400 MHz;
   - 10 MiB from the build host over TCP with the right checksum;
   - one `reboot`, back to devsh within 60 s, time to devsh logged (#47
     trend, #217 count);
   - network soak: 15 minutes idle with a ping every 10 s and a devsh
     login every minute; each loss counted with the time (#92).
5. **Release candidate:** the checklist, which should then add only what
   needs a person or a long run.

## Next

- [#257](https://github.com/redox-rpi/redox-rpi/issues/257) main red unnoticed: alert, stale-PR check, green commit for a candidate.
- [#258](https://github.com/redox-rpi/redox-rpi/issues/258) nightly `pi-test.py`: audio rate, NTP, CPU clock, TCP, reboot.
- [#259](https://github.com/redox-rpi/redox-rpi/issues/259) network loss #92/#217: nightly soak and boot count.
- [#260](https://github.com/redox-rpi/redox-rpi/issues/260) per-PR full-image QEMU check on the build host.
- [#261](https://github.com/redox-rpi/redox-rpi/issues/261) v0.4.0 rc2 checklist: desktop smoke test, console regression,
  audio tolerance, known issues in the notes.

## Later

- Known issues counted from the nightly and Pi rows automatically (log
  triage, `--crash-log`), so the "last seen" column fills itself.
