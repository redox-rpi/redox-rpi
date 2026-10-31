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

`base/0001`: when the name is taken, `register` checks the old capability
with the `fcntl` syscall, `F_GETFD`. For that the kernel looks up the
scheme without sending it a request, and answers `ENODEV` when the scheme
is gone; then the new scheme takes the name (`nsmgr: scheme <name> ended,
registering it again`). A live scheme is not asked anything and keeps its
name: `register` still gives `EEXIST` (`nsmgr: scheme <name> is taken`).

Two checks that do not work, found on the way:
- `dup` with an empty buffer never reaches the scheme: the kernel only
  copies the file description.
- `redox_rt::sys::fcntl(F_GETFD)` is answered by redox-rt from its own fd
  table, so it needs the raw syscall.

A debug kernel confirmed the kernel side is fine: after SIGKILL the
daemon's files are closed and its scheme is removed and dropped.

Not changed: a dead name is still listed in `/scheme` until it is
registered again, and namespaces forked before the restart keep the old
fd. Removing dead names from listings would need a call into every scheme
for each listing.

## Tests

`scripts/test-scheme-reregister.sh` (QEMU, needs fatfsd from experiment
`usb-fat`): fatfsd on the image's FAT partition as `t117`, SIGKILL, fatfsd
again as `t117`, list its root. That a live scheme keeps its name is not
covered: a second fatfsd on the same partition stops with `ENOLCK` before it
registers.

QEMU raspi3b, 2026-10-31, `v0.3.0-dev-671-g157dad0+x9d6b` (branch
`t117-test`: this branch with `usb-fat` merged in, for fatfsd): the
restarted fatfsd serves `t117` again, `nsmgr: scheme t117 ended,
registering it again`. Without the fix (`usb-fat` image
`v0.3.0-dev-450-gc4b9500+x87f5`): `fatfsd: /scheme/disk.mmc/0p1: File
exists (os error 17)`.
