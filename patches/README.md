# Patches

Fixes for upstream components, created with `git format-patch`. Each patch
is under the license of the project it applies to, not this repository's MIT
license.

| Directory | Project | Base revision | Apply with |
|---|---|---|---|
| [fdt/](fdt/) | [repnop/fdt](https://github.com/repnop/fdt) (MPL-2.0) | `8fa4388` (used by the Redox kernel) | `git am patches/fdt/*.patch` |
| [kernel/](kernel/) | [Redox kernel](https://gitlab.redox-os.org/redox-os/kernel) (MIT) | `6cfaba5` | `git am patches/kernel/*.patch` |

`scripts/build-kernel.sh` applies all of them automatically.
