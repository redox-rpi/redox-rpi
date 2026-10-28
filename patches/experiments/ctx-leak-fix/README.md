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

## Results (QEMU raspi3b)

| Kernel | 300× `dd if=/scheme/rand of=/dev/null bs=4096 count=1` |
|---|---|
| without the fix (`v0.3.0-dev-15-g24fbb96+x9f76`) | used +23.6 MB |
| with the fix (`v0.3.0-dev-110-gefa605d+x197a`) | used +1.2 MB (as `head`, which has no threads) |

With `ctx-leak-debug` on top: `live_kstacks` stays at 38 over 5 `dd` runs
(before: +1 per run), and the `status` handle the thread wrote to is now
closed. No `dd` is left in `ps`.

Not yet tested on the Pi. Image: homenodellm
`~/redox-rpi-sd/keep/netboot-v0.3.0-dev-110-gefa605d+x197a.img`
(`--experiment timesync --experiment rtl8152 --experiment ctx-leak-fix`).
