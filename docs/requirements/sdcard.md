# SD card (`SD`)

Topic: [sdcard](../topics/sdcard.md). Trace: [trace.md](trace.md).

### REQ-SD-1 Reliable reads and writes

The system shall read and write the SD card without losing or changing data,
also across restarts.

| Field | Value |
|---|---|
| Rationale | Users keep their files on the card. |
| Priority | must |
| Level | release |
| Status | proposed |
| Since | v0.2.0 |
| Issues | #6, #31, base `0025`–`0028`, kernel `0008` |
| Verified by | checklist v0.4.0 tests 6 and 18, row 285 (data partition on the netboot card), row 286 (root on the card, queued) |

Acceptance criteria:

1. `scripts/sd-stress.py`, short run with 2 restarts: no file lost or
   changed.
2. pi-test's `sdcard` check: a file written to `/scheme/data` reads back
   the same.

### REQ-SD-2 Small writes

Small writes shall be about as fast as large ones, and reach the card soon.

| Field | Value |
|---|---|
| Rationale | Programs write in small blocks; 28 times the data per write wore the card and was slow. |
| Priority | should |
| Level | release |
| Status | proposed |
| Since | v0.4.0 |
| Issues | #16 (closed), #155 (open), PR #73, redoxfs `0037`–`0040` |
| Verified by | checklist v0.4.0 test 18, row 285 (criterion 1); **gap** for criterion 2 |

Acceptance criteria:

1. `dd` of 1 MiB in 4 KiB blocks to `/scheme/data` takes less than 2 s.
2. Data written by a program is on the card at most 2 s later (a power cut
   after 3 s loses nothing).

### REQ-SD-3 Rename on every RedoxFS

`rename` (`mv`) shall work on every mounted RedoxFS, not only the root one.

| Field | Value |
|---|---|
| Rationale | Programs save files by writing a copy and renaming it. |
| Priority | should |
| Level | release |
| Status | proposed |
| Since | v0.4.0 |
| Issues | #57 (still open, upstream), redoxfs `0036` |
| Verified by | row 83 (dev build); **gap** in the v0.4.0 checklist |

Acceptance criteria:

1. `mv /scheme/data/a /scheme/data/b` succeeds and `b` has the content of
   `a`.
