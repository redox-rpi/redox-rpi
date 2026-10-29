# Experiment: sshd, OpenSSH server instead of devshd

For [#45](https://github.com/redox-rpi/redox-rpi/issues/45). Builds
OpenSSH 10.5p1 from Redox' recipe `net/openssh` (it is not in Redox'
prebuilt aarch64 repository) and starts `sshd` on port 22.

- `91_sshd-keys.service` runs `ssh-keygen -A` at boot: it makes the host
  keys that are missing, so only the first boot of an image takes time
  (about 2 minutes in QEMU, mostly RSA).
- `92_sshd.service` starts `/usr/bin/sshd -D -e` (sshd needs its absolute
  path to re-execute itself for each connection).
- User `sshd` and `/var/empty` for privilege separation.
- relibc `0001`: `setitimer(ITIMER_REAL)` and `getitimer` on top of the
  POSIX timer that `alarm()` uses. Without it sshd closes every connection
  ("login grace time setitimer failed").
- relibc `0002`: `setgroups` succeeds for root (the kernel has no
  supplementary groups); it returned ENOSYS and the privilege separation
  child exits on that.
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
- relibc logs `madvise`, `getsockopt(IPPROTO_TCP, TCP_NODELAY)` and
  `setsockopt(IP_TOS)` as not implemented; sshd goes on.
