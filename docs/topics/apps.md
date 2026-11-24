# Applications (own repository)

Ported software and our own applications (games such as Tux Typing,
Sopwith and Ri-li, other programs people install) live in their own
repository, [redox-rpi/apps](https://github.com/redox-rpi/apps), and are
published as packages. This repository is the OS. Maintainer decision
2026-11-24; the split and the rule of thumb are in the apps
[README](https://github.com/redox-rpi/apps#the-split).

Owner: Release & CI owns the apps repository, its build and the package
publishing. Each application's recipe belongs to its topic (Graphics for
the games and COSMIC, TCC for TinyCC, Assistant for the assistant, Network
for mdnsd).

## State

- [redox-rpi/apps](https://github.com/redox-rpi/apps): README, the list of
  what moves from here and what stays
  ([docs/migration.md](https://github.com/redox-rpi/apps/blob/main/docs/migration.md)).
- Packages are built against one OS release, since Redox has no stable
  system call ABI: `os-release` in apps pins the redox-rpi tag. The
  package repository has one path per OS release,
  `<image server>/pkg/<release>/aarch64-unknown-redox/`, and `main/` for
  development images built from main.

## In progress

- Build and publishing in apps
  ([apps#1](https://github.com/redox-rpi/apps/issues/1)):
  `scripts/build.sh` builds the recipes of apps with the release build's
  experiments and publishes them with this repository's
  `scripts/publish-pkg-repo.sh`.

## Next

- Here: experiment `pkg-repo` names the image's own release path in
  `/etc/pkg.d/40_redox-rpi`; `scripts/publish-pkg-repo.sh` writes the OS
  packages to the per-release path too.
- The moves, one application per pull request, after the v0.7.0 cut-off:
  Tux Typing, Ri-li, SDL_mixer with MOD/XM, Sopwith, COSMIC Files/Edit,
  TinyCC, the assistant, mdnsd.
- Open: whether Sopwith, COSMIC Files/Edit and TinyCC leave the release
  images (the maintainer decides).
