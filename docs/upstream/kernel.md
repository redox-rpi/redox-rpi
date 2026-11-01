# Kernel: pointers for upstream

Notes for the user, who reports to Redox. Checked against upstream on
2026-11-01: kernel `master` at `1a1295b`. (The kernel side of the netstack
leak is in [netstack.md](netstack.md).)

## A blocking pipe write returns after 64 KiB (bug)

Our [#158](https://github.com/redox-rpi/redox-rpi/issues/158).

- **What breaks:** a blocking `write()` to a pipe of more than the free
  space in the pipe (at most 64 KiB) returns a short count. POSIX says a
  blocking write returns the full count on normal completion; a short count
  is only for an interrupting signal or a non-blocking write. Programs that
  don't retry lose data: uutils `dd` (Redox' `dd`, coreutils 0.11.0) retries
  short writes only with `iflag=fullblock`, so `dd bs=1M count=16 | cmd`
  passes on 1 MiB of 16 and reports `0+16 records out`. Found in our Pi
  test 177.
- **Why:** `src/scheme/pipe.rs`, `kwrite` (line 320). It copies
  `min(MAX_QUEUE_SIZE - queued, len)` bytes (lines 343–346, queue size
  65536 at line 38) and returns that count right away (line 374). It waits
  on `write_condition` only when nothing fit (lines 379–387).
- **Reproduce:** any Redox (QEMU is enough):
  `dd if=/dev/zero bs=1M count=4 | wc -c` gives 262144 instead of 4194304,
  and dd says `0+4 records out`. With `iflag=fullblock` all bytes arrive.
- **Our fix:** kernel patch `0001` in experiment `pipe-write-all` (branch
  `pipe-write-all`, PR [#160](https://github.com/redox-rpi/redox-rpi/pull/160)). `kwrite` keeps a running total: after each
  copy it wakes the reader and, if bytes are left and the write is
  blocking, waits on `write_condition` and goes on. It returns the total
  when all bytes are in, or the bytes written so far when a signal
  interrupts the wait, the reader goes away, or the user buffer faults
  partway. A non-blocking write is unchanged. About 30 lines in that one
  function.
- **Also not POSIX, not fixed:** writes of up to `PIPE_BUF` (4096) bytes
  are not atomic when the queue has less room than that.
- **Tested:** QEMU: `dd bs=1M count=16 |` passes 16777216 bytes
  (`16+0 records out`), `dd … | sha256sum` matches the file, and
  `dd … | head -c 100` ends with `Broken pipe` instead of hanging. Pi test 180 (queued).
