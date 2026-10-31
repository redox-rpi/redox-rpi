# Experiment: pkg-hold, keep our patched packages on `pkg update`

`pkg update` replaces every installed package whose hash differs from the
upstream repository, `static.redox-os.org`. On our images that includes our
own builds of `kernel`, `relibc`, `base` and `bootloader` with the Pi
patches, so an update would take them away
([#44](https://github.com/redox-rpi/redox-rpi/issues/44)).

The experiment patches `pkgutils` (pinned in `config/upstream.lock`) with a
hold list:

- `/etc/pkg/hold` lists package names, one per line, `#` starts a comment.
- `pkg install` and `pkg update` leave an installed package from the list
  alone and print `<name> is held (etc/pkg/hold), keeping the installed version`.
- Packages that are not installed yet can still be installed.
- A line `name remote` (the remote's host, as `pkg info` shows it) still
  takes new versions of that package from that one remote. The experiment
  `pkg-repo` uses this for our own repository.

The image gets `/etc/pkg/hold` with `base`, `base-initfs`, `bootloader`,
`kernel`, `relibc` and `pkgutils`. Without `pkgutils` in the list, an update
would replace pkg with the upstream one, which has no hold support.

```bash
scripts/build-image.sh --experiment timesync --experiment pkg-hold
```

`pkg` needs the clock to be set for HTTPS, hence `timesync`.

To remove a package from the list, edit `/etc/pkg/hold`; `pkg update` then
installs the upstream version.

## Updates that replace files

Patch 0003 fixes `pkg update` itself. Upstream (pkgutils fe21bfd with
pkgar 0.2.4) replaced no file without `--nocheck`: pkgar's
`replace_diff` takes each path out of its map before it looks it up, so
every file of the new version counts as new, and the install step skips
files that exist ("already exists"). `/etc/pkg/packages.toml` still
recorded the new version. With 0003 an update skips files whose content
is the same, replaces files the old version installed when they are
unchanged on disk, and leaves files changed locally alone.

## Updates from several repositories

Patch 0004: upstream `pkg update` compares installed packages against the
index (`repo.toml`) of the first repository in `/etc/pkg.d` only, so with
our repository first it never saw newer upstream packages. With 0004 it
reads the index of every repository and takes, per package, the first
repository that lists it, the same one `pkg install` downloads from. A
repository that cannot be reached is skipped with a message. `pkg search`
then also lists the packages of every repository.
