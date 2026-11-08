# Time (`TIME`)

Topic: [time](../topics/time.md). Trace: [trace.md](trace.md).

### REQ-TIME-1 Clock from NTP

The system shall set its clock from NTP at boot when the network is up.

| Field | Value |
|---|---|
| Rationale | The Pi has no battery-backed clock; file times, TLS and logs need the right time. |
| Priority | must |
| Level | release (`timesync` is a release option since v0.2.0) |
| Status | proposed |
| Since | v0.2.0 |
| Issues | #22 (retry when the network comes late) |
| Verified by | checklist v0.4.0 test 4, row 285 |

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
| Level | release |
| Status | proposed |
| Since | v0.2.0 |
| Issues | `tools/timesync` |
| Verified by | **gap** |

Acceptance criteria:

1. Booted without a cable, `date` is not earlier than the time saved at the
   last shutdown or `reboot`, and not earlier than the build time.

### REQ-TIME-3 Set the clock by hand

The system shall let root set the clock (`clock_settime`, `settimeofday`,
`timesync -s`).

| Field | Value |
|---|---|
| Rationale | Without NTP the user must be able to set the time. |
| Priority | should |
| Level | release |
| Status | proposed |
| Since | v0.4.0 |
| Issues | #51 (still open, waiting for upstream), PR #98, relibc `0011` |
| Verified by | **gap** (no Pi row or checklist test) |

Acceptance criteria:

1. `timesync -s <time>` as root sets the clock; `date` shows the new time.
2. The same call as `user` fails with a permission error.
