# A scheme name can be registered again after its scheme ended (experiment `scheme-reregister`)

Issue [#117](https://github.com/redox-rpi/redox-rpi/issues/117). When a
scheme daemon ends, its name stays in `/scheme`. Opening it gives
`No such device`, and registering the name again fails with `File exists`.
So a driver that crashed and was restarted, or a device that came back,
could not get its old name (seen with `testnetd` for #53).

```bash
scripts/build-image.sh --experiment scheme-reregister
```

## Cause

The names are kept by the namespace manager, `initnsmgr` in `bootstrap`
(user space), not by the kernel. For each name it holds the capability fd
of the scheme. When the scheme's process ends, nothing tells the namespace
manager. It keeps the fd, and the kernel answers every call through it with
`ENODEV`, because the scheme's context is gone. `SchemeRegister::register`
refused any name already in the map with `EEXIST`.

A daemon can remove its own name when it ends normally
(`unlinkat(getns(), "/scheme/<name>")`, as fatfsd does since `usb-fat`),
but not after a crash or SIGKILL.

## Fix

`base/0001`: when the name is taken, `register` checks the old scheme with
a `dup` of its capability, which is what opening `/scheme/<name>` does too.
If the kernel answers `ENODEV`, the scheme has ended and the new one takes
the name (`nsmgr: scheme <name> ended, registering it again`). A live
scheme keeps it, and `register` still gives `EEXIST`.

Not changed: a dead name is still listed in `/scheme` until it is
registered again, and namespaces forked before the restart keep the old
fd. Removing dead names from listings would need a call into every scheme
for each listing.

## Tests

`scripts/test-scheme-reregister.sh` (QEMU, needs fatfsd from experiment
`usb-fat`): fatfsd on the image's FAT partition as `t117`, SIGKILL, fatfsd
again as `t117`, list its root.
