# Experiment: oom-test, use up all memory

Adds [`tools/oomtest`](../../../tools/oomtest) to the image. It uses up all
memory, so the kernel must end it. With kernel patch `0056` only the
process ends (SIGKILL); before, the kernel panicked with
`not yet implemented: oom` (issue #644).

```bash
scripts/build-image.sh --experiment oom-test
```

On the target:

```text
oomtest; echo $?          # page faults in user mode, 137 expected
oomtest -t 4; echo $?     # also with 4 idle threads
oomtest pipe; echo $?     # memory runs out while the kernel copies pipe data
free                      # memory is back
```

Expected: the kernel log shows `OUT OF MEMORY: killing PID ...`, the shell
prints `137` and stays usable. Exit code `2` means an allocation failed in
user space before the kernel path was reached.
