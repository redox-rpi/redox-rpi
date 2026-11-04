# base: pointers for upstream

Notes for the user, who reports to Redox. Checked against upstream on
2026-11-01: base `main` at `8424d392`. Still there. netstack, which is in
base too, has its own page: [netstack.md](netstack.md).

## A scheme name can't be registered again after its daemon ended

Our [#117](https://github.com/redox-rpi/redox-rpi/issues/117).

- **What breaks:** when a scheme daemon ends (crash, `kill -9`, a USB
  driver whose device went away), its name stays taken. Starting the daemon
  again fails with `File exists (os error 17)` when it registers the name.
  So a restarted driver, or a device that comes back, can't get its old
  name. Opening the dead name gives `No such device (os error 19)`.
- **Why:** the names are kept by the namespace manager, `initnsmgr` in
  `bootstrap/src/initnsmgr.rs` (user space), not by the kernel.
  - For each name it holds the scheme's capability fd. When the daemon
    ends, nothing tells the namespace manager, so the name stays; the
    kernel answers calls through the old fd with ENODEV.
  - `SchemeRegister::register` (line 82) answers EEXIST for any name it
    already holds (lines 84–85), without checking whether that scheme is
    still alive.
  - A debug kernel showed the kernel side is fine: after `SIGKILL` the
    daemon's files are closed and its scheme is removed.
- **Reproduce:** QEMU is enough. Start a scheme daemon under a fixed name
  (we used `fatfsd` as `t117`, and `testnetd` as `network.test0`),
  `kill -9` it, and start it again under the same name: `File exists`.
- **Our fix:** experiment `scheme-reregister`, base `0001`
  ([PR #128](https://github.com/redox-rpi/redox-rpi/pull/128)), about 15
  lines in `register`. When the name is taken, it checks the old fd with
  the raw `fcntl(F_GETFD)` syscall. The kernel looks up the scheme for that
  without sending it a request: if the scheme is gone it answers ENODEV and
  the new scheme takes the name; a live scheme isn't asked anything and
  keeps its name (EEXIST as before). Two checks that don't work: an empty
  `dup` only copies the file description, and redox-rt answers
  `fcntl(F_GETFD)` from its own table, so it has to be the raw syscall.
- **Not fixed:** a dead name still shows in `ls /scheme` until something
  registers it again. `getdents` (line 448) lists every name it holds. A
  real fix would have the namespace manager learn when a scheme ends, or
  check each name when listing.
- **Tested:** QEMU (`scripts/test-scheme-reregister.sh` on the PR branch:
  passes with the fix, `File exists` without it); Pi 3B+ test 146.
