# Experiment: sshd, OpenSSH server instead of devshd

For [#45](https://github.com/redox-rpi/redox-rpi/issues/45). Builds
OpenSSH 10.5p1 from Redox' recipe `net/openssh` (it is not in Redox'
prebuilt aarch64 repository) and starts `sshd` on port 22.

- `91_sshd-keys.service` runs `/usr/lib/redox-rpi/sshd-keys` at boot: it
  makes the host keys that are missing (`ssh-keygen -A`, about 2 minutes in
  QEMU, mostly RSA). With the SD card's data partition mounted
  (`/scheme/data`, network-boot image) the keys are kept there and copied
  to `/etc/ssh`, so they stay the same across boots; otherwise they are made
  in `/etc/ssh`.
- Root logs in with a key only (OpenSSH default `PermitRootLogin
  prohibit-password`); put the public key into
  `/home/root/.ssh/authorized_keys`. Users with a password can log in with
  it.
- `92_sshd.service` starts `/usr/bin/sshd -D -e` (sshd needs its absolute
  path to re-execute itself for each connection).
- User `sshd` and `/var/empty` for privilege separation.
- relibc `0001`: `setitimer(ITIMER_REAL)` and `getitimer` on top of the
  POSIX timer that `alarm()` uses. Without it sshd closes every connection
  ("login grace time setitimer failed").
- relibc `0002`: `setgroups` succeeds for root (the kernel has no
  supplementary groups); it returned ENOSYS and the privilege separation
  child exits on that.
- relibc `0003`: `poll`/`ppoll` merge entries that repeat an fd into one
  epoll registration (Redox keeps one per fd; the second `EPOLL_CTL_ADD`
  replaced the first). sshd polls its socket twice, for input and output;
  with nothing to write the socket was not watched at all, and the reply
  that opens a session never left.
- relibc `0004`: `ld.so` finds a program whose argv[0] starts with "-"
  (a login shell, "-ion"); it looks the program up by argv[0] and stopped
  with "failed to locate '-ion'".
- base `0024` (after the default base patches): `procmgr` sends SIGCHLD
  for an exited process without the permission check of `kill()`. The
  pre-authentication child runs as user `sshd`; its SIGCHLD to the root
  parent failed ("failed to send SIGCHLD to parent PID …: Operation not
  permitted") and `sshd-session` waited forever after the login.
- `recipes/net/openssh/0001`: no `chroot` on Redox (relibc's `chroot`
  always fails with EPERM); the pre-authentication child changes into
  `/var/empty` and drops its privileges to user `sshd`, but keeps the
  whole file system. A Redox namespace could take the place of the
  chroot later.
- `recipes/net/openssh/0002`: a login with a terminal goes on when
  `chown`/`chmod` of the PTY fail with EOPNOTSUPP (ptyd's PTYs have no
  owner or mode); before, `sshd-session` exited at once.

```bash
scripts/build-image.sh --experiment timesync --experiment rtl8152 --experiment serial-rx --experiment sshd
scripts/qemu-raspi3b.sh --image <harddrive.img> --usb-net    # ssh -p 12222 root@localhost
```

## Results

Work in progress.

- QEMU, `v0.3.0-dev-180-g657818e+x0042` (without relibc `0001`): the
  host keys are made; sshd started by hand listens on port 22 but closes
  every connection: "login grace time setitimer failed". The service
  failed with "sshd requires execution with an absolute path".
- QEMU, `v0.3.0-dev-181-g35d1df3+x0042` (with relibc `0001`): the
  service starts sshd; a connection gets through key exchange, then the
  pre-authentication child stops: `chroot("/var/empty"): Operation not
  permitted`.
- QEMU, `v0.3.0-dev-182-g934d057+x0042` (relibc `0001`, `0002`, OpenSSH
  `0001`): key login is accepted ("Accepted publickey for root"), then the
  session hangs: the SIGCHLD of the `sshd` child is refused (base `0024`).
- QEMU, `v0.3.0-dev-183-g19dcace+x0042` (with base `0024`): no SIGCHLD
  error; after the login sshd queues the session-open reply (type 91) but
  never sends it, and `sshd-session` spins at about half a core (relibc
  `0003`).
- QEMU, `v0.3.0-dev-184-g1d6e2b6+x0042` (with relibc `0003`): **key
  login works**, `ssh root@… 'echo; id; uname -a'` returns the output in
  about 17 s (QEMU, including key exchange). With a terminal (`ssh -tt`)
  the session ends at once: `chmod(/scheme/pty/N, 0600) failed: Operation
  not supported on transport endpoint` (OpenSSH `0002`). The main `sshd`
  process used 10 minutes of CPU in 30 minutes, to be checked.
- QEMU, `v0.3.0-dev-185-g891ef54+x0042` (with OpenSSH `0002`): a login
  with a terminal shows the motd, then `[ld.so]: failed to locate '-ion'`
  (relibc `0004`). Also logged: `ioctl TIOCSCTTY` not implemented, "could
  not set controlling tty".
- QEMU, `v0.3.0-dev-186-g9106a62+x0042` (with relibc `0004`): key
  login, password login (root, with `-o PermitRootLogin=yes` for the test)
  and an interactive shell work: prompt, commands, backspace, `exit`.
  **Ctrl-C does not interrupt** a running program; it does not over devshd
  either: the shell gets no controlling terminal (`ioctl TIOCSCTTY` is not
  implemented in relibc) and no foreground process group, so ptyd has
  no one to send SIGINT to. `ion` panics when the session's PTY goes away
  (after the connection closed).
- relibc logs `madvise`, `getsockopt(IPPROTO_TCP, TCP_NODELAY)` and
  `setsockopt(IP_TOS)` as not implemented; sshd goes on.

## Pi test #59 and the host key fix (2026-10-29)

Pi test #59 (`v0.3.0-dev-187-g0f28c70+x0042`) failed: `ssh-keygen -A -f
/scheme/data` made the keys but could not rename them into place ("Unable to
move … into position"), so sshd had no host keys. Rename does not work inside
a RedoxFS mounted at `/scheme/data` (`mv /scheme/data/a /scheme/data/b` gives
"No such file or directory", also on the stable image; it works in `/tmp`).
QEMU had no data partition, so the other path of the script ran there.

`sshd-keys` now copies the saved keys from `/scheme/data/etc/ssh` to
`/etc/ssh`, lets `ssh-keygen -A` make the missing ones there, and copies new
ones back with plain `cp` (`cp -p` fails: Redox cannot set file times). It
also saves the `mldsa44_ed25519` key of OpenSSH 10. Its exit status says
whether the keys exist.

QEMU, with a 16 MB RedoxFS image mounted as `data` (reproduces the rename
failure): the keys go to the data partition; after deleting `/etc/ssh/ssh_host_*`
the script brings the same keys back (same fingerprints), makes only a missing
key type, exits 0, and SSH login works.
