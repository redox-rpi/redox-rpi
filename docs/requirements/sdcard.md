# SD card (`SD`)

Topic: [sdcard](../topics/sdcard.md). Trace: [trace-v0.4.0.md](trace-v0.4.0.md).

### REQ-SD-1 Reliable reads and writes

The system shall read and write the SD card without losing or changing data,
also across restarts.

| Field | Value |
|---|---|
| Rationale | Users keep their files on the card. |
| Priority | must |
| Kind | functional |
| Level | release |
| PBS | 2.3.3, 2.4.3 |
| Status | proposed |
| Since | v0.2.0 |
| Implemented by | #6, #31, base `0025`–`0028`, kernel `0008` |
| Verification | test, automated: checklist v0.4.0 tests 6 and 18, rows 292 (v0.4.0 build) and 285 (rc1) (data partition on the netboot card), row 286 (root on the card, rc2 visit, pass) |

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
| Kind | quality |
| Level | release |
| PBS | 2.4.3 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #16 (closed), #155 (open), PR #73, redoxfs `0037`–`0040` |
| Verification | test, automated: checklist v0.4.0 test 18, rows 292 (v0.4.0 build) and 285 (rc1) (criterion 1); **gap** for criterion 2 |

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
| Kind | functional |
| Level | release |
| PBS | 2.4.3 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #57 (still open, upstream), redoxfs `0036` |
| Verification | test, automated: row 83 (dev build); **gap** in the v0.4.0 checklist |

Acceptance criteria:

1. `mv /scheme/data/a /scheme/data/b` succeeds and `b` has the content of
   `a`.
