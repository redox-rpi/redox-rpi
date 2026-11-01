# Experiment: pipe-write-all, a blocking pipe write takes all bytes

For [#158](https://github.com/redox-rpi/redox-rpi/issues/158). Kernel
patch `0001` (`src/scheme/pipe.rs`).

## The bug

A blocking write to a pipe copied what fit into the kernel's 64 KiB pipe
queue and returned that count. It waited only when nothing fit at all.
POSIX says a blocking write returns the full count on normal completion.
uutils `dd` (our `dd`, coreutils 0.11.0) retries short writes only with
`iflag=fullblock`, so `dd bs=1M … | cmd` passed on 64 KiB of every
1 MiB block and reported `0+N records out` (Pi test 177).

## The change

The write waits for the reader whenever the queue is full and goes on
until every byte is in. It returns the count written so far if a signal
interrupts it, if the reader goes away, or if the user buffer faults
partway (no data is lost or written twice). A non-blocking write still
takes what fits, and `EAGAIN` when nothing does.

Not changed: writes of up to `PIPE_BUF` (4096) bytes are still not atomic
when the queue has less room than that; POSIX asks for that too, but it
needs a separate wait condition and no program here has shown a problem.

```bash
scripts/build-image.sh --experiment pipe-write-all
```

Test: `dd if=/dev/zero bs=1M count=16 | wc -c` prints `16777216`, and dd
reports `16+0 records out`.

## Results
