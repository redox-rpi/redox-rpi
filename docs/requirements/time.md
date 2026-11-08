# Time (`TIME`)

Topic: [time](../topics/time.md). Trace: [trace.md](trace.md).

### REQ-TIME-1 Clock from NTP

The system shall set its clock from NTP at boot when the network is up.

| Field | Value |
|---|---|
| Rationale | The Pi has no battery-backed clock; file times, TLS and logs need the right time. |
| Priority | must |
| Kind | functional |
| Level | release (`timesync` is a release option since v0.2.0) |
| PBS | 2.4.4 |
| Status | proposed |
| Since | v0.2.0 |
| Implemented by | #22 (retry when the network comes late) |
| Verification | test, automated: checklist v0.4.0 test 4, row 285 |

Acceptance criteria:

1. Within 60 s after the network is up, `date` differs from the build
   host's clock by at most 2 s (proposed).
2. The same with the network cable plugged in only after boot.

### REQ-TIME-2 Clock without network

Without a network, the system shall start from the last saved time, never
from a time before the build.

| Field | Value |
|---|---|
| Rationale | A clock in 1970 breaks file times and certificates. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.4.4 |
| Status | proposed |
| Since | v0.2.0 |
| Implemented by | `tools/timesync` |
| Verification | test, automated (QEMU `raspi3b` without network, SD image, `qemu-raspi3b.sh --data`): agreed with V&V 2026-11-08, not run yet; optional in the visit without a cable |

Acceptance criteria:

1. Booted without a network, `date` is not earlier than the build time in
   `/etc/redox-rpi-release`.
2. Booted, left running, then `reboot` without a network: `date` is not
   earlier than the time saved before the reboot (`/scheme/data/timesync.last`).

### REQ-TIME-3 Set the clock by hand

The system shall let root set the clock (`clock_settime`, `settimeofday`,
`timesync -s`).

| Field | Value |
|---|---|
| Rationale | Without NTP the user must be able to set the time. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.4.1, 2.4.4 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #51 (still open, waiting for upstream), PR #98, relibc `0011` |
| Verification | test, automated (Pi row through devsh): agreed with V&V 2026-11-08, not run yet; criterion 2 moves to the visit if a command cannot be run as `user` without a person |

Acceptance criteria:

1. As root, `timesync -s @<now+3600>` sets the clock; `date` shows the new
   time within 2 s.
2. The same call as `user` fails with a permission error.
3. Plain `timesync` afterwards brings the NTP time back (criterion 1 of
   [REQ-TIME-1](#req-time-1-clock-from-ntp)).
