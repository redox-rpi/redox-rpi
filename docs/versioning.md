# Versioning of images

Every image can be traced back to exactly what went into it: the
redox-rpi commit, the upstream commits, the patches and the experiments.

## What decides an image

| Input | Where |
|---|---|
| Upstream commits (redox build system, kernel, relibc, base, bootloader, fdt) | [`config/upstream.lock`](../config/upstream.lock) |
| Our patches | [`patches/<component>/`](../patches/) |
| Experiments, successful or not | [`patches/experiments/<name>/`](../patches/experiments/), each with a README and its result |
| Image contents | [`config/raspi3bp.toml`](../config/raspi3bp.toml) (network boot), [`config/raspi3bp-sdcard.toml`](../config/raspi3bp-sdcard.toml) (SD card) |

For relibc the same state is also a git branch: `redox-rpi` in
[redox-rpi/relibc](https://github.com/redox-rpi/relibc) is the pinned
commit (branch `upstream`) plus `patches/relibc/`, and `exp/<name>` adds a
relibc experiment. `git format-patch upstream..redox-rpi` gives the patch
files again. The patch files in this repository are what the build uses.

Packages other than the core components above come as binaries from the
Redox package server and are not pinned; the manifest records their
versions and hashes.

## Rules in `scripts/build-image.sh`

- Every source is checked out at its commit from `config/upstream.lock`.
- Experiments are only applied with `--experiment <name>`. Besides patches,
  an experiment may have a `config.toml` that adds packages or files to the
  image; the build includes it after the image's own config.
- Before building, each recipe source is compared with its commit plus
  patches. A direct edit in `upstream/redox/recipes/core/*/source` stops the
  build: turn it into a patch, or delete the source to drop it.
- Uncommitted changes to `patches/`, `config/`, `scripts/` or `tools/` stop
  the build too.
- `--allow-dirty` builds anyway: the build ID gets `-dirty.<hash>` and the
  differences are saved as `build/manifests/<image>-<build ID>.diff`.

## Combining experiments

Experiments are applied after the default patches, in the order of the
`--experiment` options; the base patch numbers in
[patches/README.md](../patches/README.md) say which order works. The release candidate v0.2.0 is:

```bash
scripts/build-image.sh [--sdcard] --experiment ldso-diagnostics \
    --experiment timesync --experiment sdhost-driver --experiment rtl8152
```

## Release names

- Releases are named `v0.x.y`, without `-alpha` or another suffix. Until
  1.0, a release with new features raises `x` (v0.2.0 → v0.3.0), a release
  with only fixes raises `y` (v0.2.0 → v0.2.1).
- Test candidates get `-rcN` (v0.2.0-rc2). The suffix appears only on test
  images, never on a release or its tag.
- How mature the release is, is said once in the release notes and in the
  README, for example: "Status: experimental (alpha)".
- v0.1.0-alpha keeps its name; it was the last release with the suffix.

## Build ID

```text
v0.1.0-alpha-61-g90ef0ee+xd025-dirty.0f3e9a1b2c4d
└─ git describe ───────┘└exp┘└─ unrecorded changes
```

- `+x` and four hex digits: a hash of the experiments, sorted, so the same
  set gives the same ID in any order. Without experiments the part is left
  out. Which experiments they are is in the manifest, in
  `EXPERIMENTS` in `/etc/redox-rpi-release`, and on the line below the build
  ID above the login prompt.
- `-dirty.` and a hash: only with `--allow-dirty`.

It is shown above the login prompt and in `/etc/redox-rpi-release`.

Images built before 2026-10-28 have the experiment names in the ID itself,
for example `v0.1.0-alpha-28-gb5c59b2+ldso-diagnostics+sdhost-driver`. The
scripts treat both forms the same way: they find a manifest by the hash of
`harddrive.img`, not by its name.

## Manifest

The build writes the manifest twice:

- **In the image:** `/etc/redox-rpi-manifest`, with the build ID, config,
  commit, experiments, upstream commits and a hash of every applied patch.
- **Next to the image:** `build/manifests/<image>-<build ID>.toml`, which
  also has the hashes of `harddrive.img`, the kernel, `libc.so` and the
  bootloader, the GNU build ID of `libc.so` and every package with version,
  source commit and hash.

`scripts/netboot.sh` puts the matching manifest into the TFTP directory as
`manifest.toml`. `scripts/make-sdcard.sh --local` writes it as
`<image>.manifest.toml`. Both find it by the hash of `harddrive.img`.

## Which build is running?

On the Pi:

```bash
cat /etc/redox-rpi-release      # build ID and experiments
cat /etc/redox-rpi-manifest
sha256sum /usr/lib/libc.so
```

Compare the hash with the `[outputs]` section of the manifests in
`build/manifests/`. `libc.so` also has a GNU build ID (relibc patch `0006`).
It is a hash of the linked code, so the same code always gets the same ID.
On Linux, read it with `readelf -n libc.so`.
