# Experiment: oom-test, use up all memory

Adds [`tools/oomtest`](../../../tools/oomtest) to the image. It uses up all
memory, so the kernel must end it. It is a test experiment
(`config/test-experiments.conf`, `test-only`): every test image has it for
the `oom` check of `scripts/regression-set.py` (#676); releases never do. With kernel patch `0056` only the
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
oomtest fork-shared 1     # fork with a shared mapping works, 0 expected
oomtest fork-shared       # fork of 2 GiB untouched shared memory (#679)
```

Expected: the kernel log shows `OUT OF MEMORY: killing PID ...`, the shell
prints `137` and stays usable. Exit code `2` means an allocation failed in
user space before the kernel path was reached.
