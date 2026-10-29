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
- relibc logs `setgroups` and `madvise` as not implemented; sshd goes on.
