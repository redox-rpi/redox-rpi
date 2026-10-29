# Experiment: pkg-repo, redox-rpi's own package repository

`pkg` normally knows only `static.redox-os.org`, which has the upstream
builds. Our builds of `kernel`, `relibc`, `base`, `bootloader` and
`pkgutils` (with the Pi patches) and our own tools (`redox-fatfs2`,
`timesync`, `reboot`) are not there
([#44](https://github.com/redox-rpi/redox-rpi/issues/44)).

`scripts/publish-pkg-repo.sh` publishes them from the last
`scripts/build-image.sh` run on the build host:

- `http://homenodellm:8080/pkg/id_ed25519.pub.toml`: the public key.
- `http://homenodellm:8080/pkg/aarch64-unknown-redox/`: `repo.toml`,
  `<name>.pkgar`, `<name>.toml`.

The packages are signed again with a fixed key (the cookbook's key differs
in every clone). The secret key is in `~/.config/redox-rpi/pkg-key/` on
the build host. `devshd` is never published.

This experiment adds `/etc/pkg.d/40_redox-rpi` with
`http://192.168.178.120:8080/pkg`. `pkg` reads the files in
`/etc/pkg.d` in name order, so it asks our repository before
`50_redox`:

- `pkg install <name>` takes our build if we publish one, else the
  upstream one.
- `pkg update` compares against the first repository's index only
  (pkgutils has no multi-repository update yet), so it updates our
  packages from our repository and leaves upstream packages alone.

Use it together with `pkg-hold`:

```bash
scripts/build-image.sh --experiment timesync --experiment pkg-hold --experiment pkg-repo
scripts/publish-pkg-repo.sh
```

Publish again after each build that should reach the Pis; the index keeps
packages published earlier.
