# Experiment: mmap-stop, repro for the scheme mmap todo!s

Adds [`tools/mmapstop`](../../../tools/mmapstop) to the image (#678). A
child process maps a file with `MAP_PRIVATE` and `PROT_WRITE` and writes
to it. The write fault makes the kernel ask the scheme for the page and
blocks the child (`AwaitingMmap`). The parent then signals the child.

```bash
scripts/build-image.sh --experiment mmap-stop
```

On the target:

```text
mmapstop none; echo $?    # does the write block? 0 no, 3 yes
mmapstop -s none; echo $? # the same with MAP_SHARED
mmapstop kill; echo $?    # SIGKILL to the blocked child
mmapstop stop; echo $?    # SIGSTOP, then SIGKILL
```

Exit code 3 means the child blocked and was reaped, 4 that it could not be
reaped (a hang). A kernel panic (`not yet implemented`) is the bug.
This is a test tool only, never for release images.
