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

## Patch numbers

- **On a branch, new patches stay an experiment:**
  `patches/experiments/<name>/<component>/`. Their numbers there may
  repeat other branches' numbers; the `exp-<name>-` prefix keeps them
  apart.
- **A number in `base/`, `kernel/`, `relibc/` and the other default
  directories is given only at the merge to `main`**, by whoever merges:
  the next free number in that directory, in the order the patches must
  apply. The merger moves the files (`git mv`), notes in the experiment's
  README that it is now default patches, and checks that the default build
  still boots (QEMU, then the nightly run).
- A branch that already moved its patches to a default directory keeps
  them there; the merger renumbers them if the numbers are taken.

Experiments are applied after the default patches, in the order of the
`--experiment` options; their files get an `exp-<name>-` prefix in the
recipe, so equal numbers do not collide.

**What each patch does** is in its commit message (the patch file's
header). To list the numbers in use and what they do:

```sh
grep -H -m1 '^Subject:' patches/base/*.patch
```

The notes up to 2026-10-31, including which experiment each default patch
came from, are in [history.md](history.md).

**Merged experiments:** an experiment directory with only a README is one
that became default patches; its README says which. `--experiment` still
accepts the name and does nothing.

**Lists that nobody edits:** to keep pull requests from conflicting with
each other, no shared list names patches or programs:

- `scripts/build-image.sh` pins every core recipe that has a line in
  `config/upstream.lock`, and patches those with a directory here
  (`patches/<recipe>/`) or in a selected experiment. A patched recipe needs
  a lock line. A core recipe without one builds the newest upstream commit
  each time (#145), so every recipe built from source belongs in the lock.
- It builds every program in `tools/<name>/` that has a `recipe.toml` (the
  `[build]` part of its recipe; it adds the `[source]` part). Whether the
  image contains the program is decided by the image config or the
  experiment's `config.toml`.

Reported upstream (drop the patch once the pinned commit has the fix):

| Patch | Upstream report |
|---|---|
| `relibc/0004` x1/x2 swap in the aarch64 signal entry | [relibc#309](https://gitlab.redox-os.org/redox-os/relibc/-/work_items/309) |

The upstream commits the patches apply to are in
[`config/upstream.lock`](../config/upstream.lock). See
[docs/versioning.md](../docs/versioning.md).

`scripts/build-kernel.sh` applies all of them automatically.
