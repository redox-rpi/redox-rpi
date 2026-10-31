# Experiment: pkg-persist, packages that survive a reset

Network-boot and direct-boot images run their root filesystem from RAM:
whatever `pkg install` puts into `/` is gone after a reset
([#44](https://github.com/redox-rpi/redox-rpi/issues/44)). The SD card's data partition, mounted at
`/scheme/data` by `60_data.service`, survives.

- pkgutils patch 0001 adds `pkg --root <dir>`: pkg installs into that
  directory and keeps its package list, `etc/pkg.d` and `etc/pkg/hold`
  there.
- [`tools/pkg-persist`](../../../tools/pkg-persist) uses it with
  `/scheme/data/pkg` and copies the installed files into `/`, at once and
  at every boot (`61_pkg-persist.service`). Copies, not links: Redox does
  not follow symlinks into another scheme.

```bash
scripts/build-image.sh --experiment timesync --experiment pkg-hold --experiment pkg-repo --experiment pkg-persist
```

On the Pi:

```bash
pkg-persist install nano    # into /scheme/data/pkg, then copied into /
pkg-persist list
pkg-persist update
pkg-persist remove nano     # the copies in / stay until the next reset
```

Rules:

- The image wins: a file is copied only if `/` does not have it, or if
  pkg-persist copied it itself during this boot (so `update` replaces it).
  Dependencies that are already in the image are installed on the data
  partition too, but the image's files are used.
- Before every pkg run the data partition gets the image's `/etc/pkg.d`
  and `/etc/pkg/hold`, so the same repositories and holds apply.
- Plain `pkg install` still installs into `/` only, as before.

Also in this experiment:

- redoxfs patch 0001: rename failed with "No such file or directory" on any
  RedoxFS mounted below `/scheme/<name>` (only the root filesystem worked),
  and pkg renames every file it installs. Built from source and held.
- pkgutils patch 0002: when a download fails because the certificate is
  "not valid yet" (or expired), pkg says what year the clock is at and to
  set the clock first, instead of only the TLS error.

The boot copy is a `oneshot` service, so a service that names
`61_pkg-persist.service` in `requires_weak` starts only once the packages
are in `/`. The login prompt does not wait: `30_getty_debug` depends only
on `00_base.target`, so `redox login:` usually comes before the copy line
(the line ends with the time the copy took). A program from the data
partition typed in the first moments after login may not be there yet.
Init has read its service files by then, so a service file that a package
brings does not start at boot.

Without a data partition (network boot without a card) `pkg-persist`
fails at the pkg step and `restore` does nothing.

## QEMU test (2026-10-30)

Image `v0.3.0-dev-459-gcc84a42+xc526` (`timesync`, `pkg-hold`, `pkg-repo`,
`pkg-persist`, `sigreturn-svc`), QEMU SD card with a fourth partition made
with `redoxfs-mkfs`, mounted by hand as `data` (in QEMU the card's second
partition is the EFI one, so `60_data` fails there).

- Before the redoxfs patch: `mv /scheme/data/a /scheme/data/b` fails with
  "No such file or directory", and so did `pkg-persist install nano`
  (pkg's "Renaming file" step). `mv` on the root filesystem worked.
- With it: `mv` works; `pkg-persist install nano` installs nano, ncursesw
  and terminfo onto the data partition, copies 214 files into `/` (2 kept
  from the image) and `nano --version` prints GNU nano 9.2.
- Reset: the same card with partitions 1 to 3 written back from the image
  (QEMU's root is on the card, the Pi's is in RAM), data partition kept.
  `/usr/bin/nano` is gone, `pkg-persist restore` copies the 214 files
  again without network, and nano runs.

## Pi test (2026-10-31)

Test 124 with `v0.3.0-dev-462-g3783750+xc526` passes: `mv` on
`/scheme/data` works, `pkg-persist install nano` copies 214 files and nano
runs, and after `reboot` the boot service copies the same 214 files and
nano runs again. A second `restore` copies nothing. The boot copy line came
after `redox login:`: the login prompt does not depend on the data
partition (see above). The service is a `oneshot` since, for services
that depend on it.

Recheck, test 127 with `v0.3.0-dev-464-g3835c4d+xc526` (oneshot): passes.
The boot copy of the same 214 files takes 3940 ms (the install's copy
3736 ms) and now comes right after `redox login:` and before both
timesync lines, so the services after it start about 4 s later when
packages are on the data partition. nano runs after the reboot.
