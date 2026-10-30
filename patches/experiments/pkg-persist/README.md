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

Without a data partition (network boot without a card) `pkg-persist`
fails at the pkg step and `restore` does nothing.
