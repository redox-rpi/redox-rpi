# Native C compiler (`TCC`)

Topic: [tcc](../topics/tcc.md). Trace: [trace-v0.4.0.md](trace-v0.4.0.md).

### REQ-TCC-1 Compile and run C on the Pi (beta)

The release image shall compile, link and run a C program on the Pi
itself with TinyCC (`tcc`).

| Field | Value |
|---|---|
| Rationale | A compiler on the device lets users write programs without a cross toolchain. |
| Priority | may |
| Kind | functional |
| Level | beta (in the release image since v0.5.0; dynamic linking only) |
| PBS | 2.5.3 |
| Status | proposed |
| Since | v0.5.0 (beta; experiment before) |
| Implemented by | #247, PR #249, #293, PR #297 (release image) |
| Verification | test, automated: row 278 (dev build); v0.5.0: row 414 (`tcc -run`, tcc script) |

Acceptance criteria:

1. `tcc -o hello hello.c && ./hello` prints the expected line.
2. `tcc -run hello.c` prints the same.
4. A file starting with `#!/usr/bin/tcc -run` runs as a script.
5. The examples in `/usr/share/tcc-tests/` compile and run.
3. Static linking (`tcc -static`) is a known limit (TLS descriptor
   relocations in relibc's `libc.a`) and not a criterion yet.

Change for v0.6.0 (Requirements agent, 2026-11-16): tcc is in the release
image as beta since v0.5.0 (#293, PR #297). Level experiment → beta;
criteria 4 and 5 added from the v0.5.0 release notes. Impact: release
checklist (a tcc step), row 414's tcc steps, the anchor of this entry.
