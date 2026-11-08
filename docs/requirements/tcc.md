# Native C compiler (`TCC`)

Topic: [tcc](../topics/tcc.md). Trace: [trace.md](trace.md).

### REQ-TCC-1 Compile and run C on the Pi (experiment)

With the experiment `tcc`, the system shall compile, link and run a C
program on the Pi itself.

| Field | Value |
|---|---|
| Rationale | A compiler on the device lets users write programs without a cross toolchain. |
| Priority | may |
| Kind | functional |
| Level | experiment (not in the release image) |
| PBS | 2.5.3 |
| Status | proposed |
| Since | — (draft PR #249) |
| Implemented by | #247, PR #249 (draft) |
| Verification | test, automated: row 278 (dev build) |

Acceptance criteria:

1. `tcc -o hello hello.c && ./hello` prints the expected line.
2. `tcc -run hello.c` prints the same.
3. Static linking (`tcc -static`) is a known limit (TLS descriptor
   relocations in relibc's `libc.a`) and not a criterion yet.
