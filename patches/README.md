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
  apply. The merger moves the files (`git mv`), updates the table below and
  checks that the default build still boots (QEMU, then the nightly run).
- A branch that already moved its patches to a default directory keeps
  them there; the merger renumbers them if the numbers are taken.

Experiments are applied after the default patches, in the order of the
`--experiment` options; their files get an `exp-<name>-` prefix in the
recipe, so equal numbers do not collide. The base patch numbers say what
each one expects to come before it:

| Number | Where | What |
|---|---|---|
| `0001`–`0016`, `0018`, `0020`–`0028`, `0030` | `base/` | default patches; `0018` is `rpi-firmwared`, `0020` RTL8152/RTL8153 Ethernet in `dwc2d` (was experiment `rtl8152`), `0021` the ARM clock and throttle watch of `rpi-firmwared`, `0022` `dwc2d` waits for its interrupt (was experiment `dwc2-interrupts`, with kernel `0004`–`0006`), `0023` `dwc2d` picks the network adapter with a link, `0024` getty keeps input the PTY does not take yet (was experiment `serial-rx`, with kernel `0007`; #28, #32), `0025`–`0028` the sdhost driver `bcm2835-sdhostd`, started at boot instead of `bcm2835-sdhcid` (were experiments `sdhost-driver` and `sdhost-boot`, #6), `0030` no PCI and audio services on the Pi (was experiment `pi-clean-boot`; touches only init, `hwd` and unit files, so it applies with any experiment) |
| `0001`–`0008` | `kernel/` | default patches; `0008` frees a thread that ends itself when its syscall returns (was experiment `ctx-leak-fix`, #31) |
| `0029` | `experiments/sdhost-stats/base/` | transfer statistics at boot |
| `0036` | `redoxfs/` | `rename` within a non-root RedoxFS such as `/scheme/data` named by an absolute path (was experiment `redoxfs-rename`, #57); RedoxFS is pinned in `config/upstream.lock` like the other recipes |
| `0037`–`0039` | `redoxfs/` | small consecutive writes of a handle are collected into whole records and written at the latest after 2 s, so a copy in 4 KiB blocks writes 1.3 times its data instead of 28 times (was experiment `redoxfs-write-coalesce`, #16; on top of `0036`) |

Experiments apply after all default patches. Combinations that build:
<<<<<<< HEAD
`sdhost-driver` (with `sdhost-boot` or `sdhost-stats`),
`timesync`, `ldso-diagnostics`. `rtl8152` and `dwc2-interrupts` are
default patches now, and so are `serial-rx`, `pi-clean-boot`, `redoxfs-rename` and `redoxfs-write-coalesce`; the names are still accepted
=======
`sdhost-stats`, `timesync`, `ldso-diagnostics`.
`rtl8152`, `dwc2-interrupts`, `serial-rx`, `pi-clean-boot`, `sdhost-driver`,
`sdhost-boot`, `ctx-leak-fix` and `redoxfs-rename` are default patches now; the names are still accepted
>>>>>>> origin/redoxfs-rename
and do nothing.

Reported upstream (drop the patch once the pinned commit has the fix):

| Patch | Upstream report |
|---|---|
| `relibc/0004` x1/x2 swap in the aarch64 signal entry | [relibc#309](https://gitlab.redox-os.org/redox-os/relibc/-/work_items/309) |

The upstream commits the patches apply to are in
[`config/upstream.lock`](../config/upstream.lock). See
[docs/versioning.md](../docs/versioning.md).

`scripts/build-kernel.sh` applies all of them automatically.
