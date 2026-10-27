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

## Build ID

```text
v0.1.0-alpha-12-g1a2b3c4+ldso-diagnostics-dirty.0f3e9a1b2c4d
└─ git describe ────────┘└─ experiments ─┘└─ unrecorded changes
```

It is shown above the login prompt and in `/etc/redox-rpi-release`.

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
cat /etc/redox-rpi-release
cat /etc/redox-rpi-manifest
sha256sum /usr/lib/libc.so
```

Compare the hash with the `[outputs]` section of the manifests in
`build/manifests/`. `libc.so` also has a GNU build ID (relibc patch `0006`).
It is a hash of the linked code, so the same code always gets the same ID.
On Linux, read it with `readelf -n libc.so`.
