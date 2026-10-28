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

Experiments are applied after the default patches, in the order of the
`--experiment` options; their files get an `exp-<name>-` prefix in the
recipe, so equal numbers do not collide. The base patch numbers say what
each one expects to come before it:

| Number | Where | What |
|---|---|---|
| `0001`–`0016`, `0018`, `0021` | `base/` | default patches; `0018` is `rpi-firmwared`, `0021` its ARM clock and throttle watch (after the experiments' `0017`–`0020`, which it does not touch) |
| `0017`, `0018` | `experiments/sdhost-driver/base/` | sdhost driver, high speed mode and statistics (`0018` here applies after `base/0018`) |
| `0018` | `experiments/sdhost-boot/base/` | start sdhost at boot instead of sdhcid (after `sdhost-driver`) |
| `0019` | `experiments/sdhost-stats/base/` | transfer statistics at boot (after `sdhost-driver`) |
| `0020` | `experiments/rtl8152/base/` | RTL8152/RTL8153 Ethernet in `dwc2d` |
| `0021` | `experiments/pi-clean-boot/base/` | no PCI and audio services on the Pi |
| `0022` | `experiments/dwc2-interrupts/base/` | `dwc2d` waits for its interrupt (after `rtl8152`); kernel `0004`–`0006` in the same experiment |
| `0023`, kernel `0008` | `experiments/cpu-ondemand/` | `rpi-firmwared` governor: the ARM clock follows the load (after `base/0021`); kernel: CPU times in `sys:stat` without rounding |

Combinations that build: `sdhost-driver` + `rtl8152` (release candidate
v0.2.0-alpha, in this order), each with `ldso-diagnostics` and `timesync`.
`rtl8152` + `pi-clean-boot` + `dwc2-interrupts`, in this order (test #5 on
the Pi).

Reported upstream (drop the patch once the pinned commit has the fix):

| Patch | Upstream report |
|---|---|
| `relibc/0004` x1/x2 swap in the aarch64 signal entry | [relibc#309](https://gitlab.redox-os.org/redox-os/relibc/-/work_items/309) |

The upstream commits the patches apply to are in
[`config/upstream.lock`](../config/upstream.lock). See
[docs/versioning.md](../docs/versioning.md).

`scripts/build-kernel.sh` applies all of them automatically.
