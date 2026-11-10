# Stability (`STAB`)

Topics: [stability](../topics/stability.md), [relibc](../topics/relibc.md).
Trace: [trace-v0.4.0.md](trace-v0.4.0.md).

### REQ-STAB-1 Programs start every time

Dynamically linked programs shall start every time.

| Field | Value |
|---|---|
| Rationale | Random start failures made v0.1.0 unusable. |
| Priority | must |
| Kind | quality |
| Level | release |
| PBS | 2.2.1, 2.4.1 |
| Status | proposed |
| Since | v0.3.0 |
| Implemented by | #1, #5, #12, #127, relibc `0004`, `0007`, kernel `0003` |
| Verification | test, automated: checklist v0.4.0 test 1, rows 292 (v0.4.0 build) and 285 (rc1) (0 of 1000) |

Acceptance criteria:

1. pi-test's `starts` check: 0 of 1000 starts fail, at 1400 MHz.
2. No crash dump on the serial console during the starts.

### REQ-STAB-2 Pipes keep all data

A write into a pipe shall deliver all its bytes.

| Field | Value |
|---|---|
| Rationale | Shell pipelines lost data. |
| Priority | must |
| Kind | functional |
| Level | release |
| PBS | 2.2.1 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #158, PR #160, kernel `0010` |
| Verification | test, automated: checklist v0.4.0 test 20, rows 292 (v0.4.0 build) and 285 (rc1) |

Acceptance criteria:

1. `dd if=/dev/zero bs=64k count=256 | cat > /scheme/data/p` leaves a file
   of 16 MiB.

### REQ-STAB-3 C library regressions

The C library (relibc) shall pass its test suite on the Pi's target, with
no failure beyond the known list.

| Field | Value |
|---|---|
| Rationale | relibc carries most of our fixes; its tests catch regressions before the Pi does. |
| Priority | must |
| Kind | functional |
| Level | release |
| PBS | 2.4.1 |
| Status | proposed |
| Since | v0.3.0 |
| Implemented by | #13, #108 (known failure `pthread/exit`) |
| Verification | test, QEMU: checklist v0.4.0 test 15 (QEMU, build agent): pass |

Acceptance criteria:

1. `scripts/relibc-tests.sh` with the release options: `sys_socket/unixrecv`,
   `unixrecvfrom` and `unixnonblock` pass.
2. Every other failure is on the known list in the checklist.
