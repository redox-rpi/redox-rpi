# ctx-leak-debug

Debug kernel for [#31](https://github.com/redox-rpi/redox-rpi/issues/31): the
kernel keeps about 76 KB after each run of a program with several threads
(`dd`), which ends in an `oom` panic after about 1500 runs.

The kernel patch counts the kernel stacks that exist and logs one line per
context exit:

    ctxexit <name> pid <pid> files_refs <n> live_kstacks <n> contexts <n>

`files_refs` is the number of references to the file table when the context
exits; only the context that holds the last one closes the files.

Not for normal images: it logs every exit.
