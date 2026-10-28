# ctx-leak-fix

Fix for [#31](https://github.com/redox-rpi/redox-rpi/issues/31): the kernel
kept about 76 KB after each run of a program with a thread that ends itself
(`dd` from coreutils), and ran out of memory after about 1500 runs (Pi test
#23, `sd-stress.py`).

## Cause

A thread ends itself (relibc `exit_this_thread`) by writing `ForceKill` to
its own `status` handle of the proc scheme. The kernel called
`exit_this_context` inside that write. `exit_this_context` does not return,
so the `Arc` of the file description that `sys_write` holds was never
dropped. The description was never closed, so its proc handle stayed in
`HANDLES`, and the handle kept the dead context alive with its 64 KiB kernel
stack.

Found with [`ctx-leak-debug`](../ctx-leak-debug): `live_kstacks` grew by one
per `dd`, and of its three contexts the one that ended itself kept one
reference after procmgr closed its handles.

## Fix

The kernel patch sets `being_sigkilled` instead (as for a thread killed by
another one), and the syscall entry calls `exit_this_context` after the
syscall, when the syscall path has dropped its references. The code is not
aarch64-specific, so x86_64 should have the same leak.
