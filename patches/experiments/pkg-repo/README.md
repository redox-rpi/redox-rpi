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
- `pkg update` compares each installed package against the first
  repository that has it (pkg-hold patch 0004; upstream pkgutils reads
  only the first repository's index): our packages against ours,
  everything else against `static.redox-os.org`.

It also replaces the hold list of `pkg-hold` with entries like
`kernel 192.168.178.120`: our patched packages are updated from our
repository and never from `static.redox-os.org`.

Use it together with `pkg-hold`, listed after it:

```bash
scripts/build-image.sh --experiment timesync --experiment pkg-hold --experiment pkg-repo
scripts/publish-pkg-repo.sh
```

Publish again after each build that should reach the Pis; the index keeps
packages published earlier. `pkg update` compares checksums, not versions
or dates: a Pi with a newer build than the repository gets the
repository's older packages. So publish every build that goes onto Pis.

## QEMU test (2026-10-30)

Image E `v0.3.0-dev-195-gb17a05f+xb43c` (`timesync`, `pkg-hold` with
0004, `pkg-repo`); the repository still had build D (2026-10-29).

- `pkg search reboot` lists packages of both repositories.
- `pkg update --yes` updates 17 packages: the 15 upstream ones that differ
  from our pin (coreutils, ion, redoxfs, uutils, …) from
  `static.redox-os.org`, and `base` and `pkgutils` from our repository,
  which allows them (`base 192.168.178.120` in the hold list). Nothing is
  ignored, no "already exists".
- Kernel and `libc.so` unchanged, `reboot` stays 0.1.2 (same in both).
  `/usr/bin/pkg` changed to build D's, which lacks 0004: the older
  repository won, see above.
- `ls --version` (uutils 0.11.0 from upstream) works; a second
  `pkg update --yes` has nothing to do.

## Pi test (2026-10-30)

Test 115 on a Pi 3B+, image `v0.3.0-dev-456-gfe96015+x204a` (this branch
plus `sigreturn-svc` from PR #61, so that Redox's prebuilt programs keep
x18 over signals), repository published from the same build: `pkg update
--yes` updated the 15 upstream packages and none of ours; kernel,
`libc.so`, `pkg` and `reboot` unchanged; afterwards 0 of 300 starts
failed, `pkg install nano` and `pkg install bash` work, and bash ran 200
child processes. Pass.
