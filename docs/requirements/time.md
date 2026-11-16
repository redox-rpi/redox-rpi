# Time (`TIME`)

> **Moved to Feature issues (2026-11-16).** Each requirement below is now a
> Feature issue in redox-rpi, and the issue is the record ([README](README.md#feature-issues)).
> This file stays until the links to it are moved; do not edit it.

Topic: [time](../topics/time.md). Trace: [trace-v0.4.0.md](trace-v0.4.0.md).

### REQ-TIME-1 Clock from NTP

Feature issue: [#508](https://github.com/redox-rpi/redox-rpi/issues/508)

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
| Verification | test, automated: checklist v0.4.0 test 4, rows 292 (v0.4.0 build) and 285 (rc1) |

Acceptance criteria:

1. Within 60 s after the network is up, `date` differs from the build
   host's clock by at most 2 s (proposed).
2. The same with the network cable plugged in only after boot.

### REQ-TIME-2 Clock without network

Feature issue: [#509](https://github.com/redox-rpi/redox-rpi/issues/509)

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

Feature issue: [#51](https://github.com/redox-rpi/redox-rpi/issues/51)

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
| Implemented by | #51 (open: the upstream part), PR #98, relibc `0011` |
| Verification | test, automated (Pi row through devsh, `user` step piped into `su user`): agreed with V&V 2026-11-08; dry run on stable 748 passed all three criteria, not yet a row on a candidate |

Acceptance criteria:

1. As root, `timesync -s '@<now+3600>'` (single quotes: ion reads a bare
   `@…` as an array) sets the clock; `date` shows the new time within 2 s.
2. As `user` (`echo 'timesync -s "YYYY-MM-DD HH:MM:SS"; echo rc=$?' | su
   user`) it fails with "Operation not permitted", exit status 1, and the
   clock is unchanged.
3. Plain `timesync` afterwards brings the NTP time back (criterion 1 of
   [REQ-TIME-1](#req-time-1-clock-from-ntp)).
