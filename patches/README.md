# Patches

Fixes for upstream components, created with `git format-patch`. Each patch
is under the license of the project it applies to, not this repository's MIT
license.

| Directory | Project | Apply with |
|---|---|---|
| [fdt/](fdt/) | [repnop/fdt](https://github.com/repnop/fdt) (MPL-2.0) | `git am patches/fdt/*.patch` |
| [kernel/](kernel/) | [Redox kernel](https://gitlab.redox-os.org/redox-os/kernel) (MIT) | `git am patches/kernel/*.patch` |
| [base/](base/) | [Redox base](https://gitlab.redox-os.org/redox-os/base) (MIT): drivers, init | `git am patches/base/*.patch` |
| [bootloader/](bootloader/) | [Redox bootloader](https://gitlab.redox-os.org/redox-os/bootloader) (MIT) | `git am patches/bootloader/*.patch` |
| [relibc/](relibc/) | [relibc](https://gitlab.redox-os.org/redox-os/relibc) (MIT): C library, dynamic loader | `git am patches/relibc/*.patch` |
| [experiments/](experiments/) | Patches tried while debugging, with their results; applied only with `scripts/build-image.sh --experiment <name>` | |

The upstream commits the patches apply to are in
[`config/upstream.lock`](../config/upstream.lock). See
[docs/versioning.md](../docs/versioning.md).

`scripts/build-kernel.sh` applies all of them automatically.
