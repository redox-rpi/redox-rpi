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
| `0001`–`0016`, `0018`, `0020`–`0028`, `0030`, `0037`, `0040`, `0045`–`0048`, `0056` | `base/` | default patches; `0018` is `rpi-firmwared`, `0020` RTL8152/RTL8153 Ethernet in `dwc2d` (was experiment `rtl8152`), `0021` the ARM clock and throttle watch of `rpi-firmwared`, `0022` `dwc2d` waits for its interrupt (was experiment `dwc2-interrupts`, with kernel `0004`–`0006`), `0023` `dwc2d` picks the network adapter with a link, `0024` getty keeps input the PTY does not take yet (was experiment `serial-rx`, with kernel `0007`; #28, #32), `0025`–`0028` the sdhost driver `bcm2835-sdhostd`, started at boot instead of `bcm2835-sdhcid` (were experiments `sdhost-driver` and `sdhost-boot`, #6), `0030` no PCI and audio services on the Pi (was experiment `pi-clean-boot`; touches only init, `hwd` and unit files, so it applies with any experiment), `0037` `dwc2d` asks again for a USB reply shorter than the fields it reads, instead of panicking (was experiment `dwc2d-short-reply`; test 98, test 105), `0040` the `rpi-firmwared` governor, the ARM clock follows the load (was experiment `cpu-ondemand`, with kernel `0040`; #23, Pi test #93), `0045` GPIO driver `bcm2835-gpiod`, `0046` `rpi-firmwared` `power_led` (after `0040`, same lines), `0047` I2C driver `bcm2835-i2cd`, `0048` SPI driver `bcm2835-spid` (were experiments `gpio`, `i2c`, `spi`; #38, #39, #42, Pi test #99), `0056` netstack lists its `netcfg` directories, so `ls /scheme/netcfg/ifaces` works (#105) |
| `0001`–`0008`, `0040`, `0057` | `kernel/` | default patches; `0008` frees a thread that ends itself when its syscall returns (was experiment `ctx-leak-fix`, #31), `0040` CPU times in `sys:stat` without rounding (with base `0040`, #23), `0057` unmaps the path page of an open at the root of a scheme (`/scheme/tcp`, `/scheme/rand`; 4 KiB per open stayed in the daemon, #88) |
| `0029` | `experiments/sdhost-stats/base/` | transfer statistics at boot |

Experiments apply after all default patches. Combinations that build:
`sdhost-stats`, `timesync`, `ldso-diagnostics`.
`rtl8152`, `dwc2-interrupts`, `serial-rx`, `pi-clean-boot`, `sdhost-driver`,
`sdhost-boot`, `ctx-leak-fix`, `dwc2d-short-reply`, `cpu-ondemand`, `gpio`, `i2c` and `spi` are default patches now; the names are still accepted
and do nothing.

Reported upstream (drop the patch once the pinned commit has the fix):

| Patch | Upstream report |
|---|---|
| `relibc/0004` x1/x2 swap in the aarch64 signal entry | [relibc#309](https://gitlab.redox-os.org/redox-os/relibc/-/work_items/309) |

The upstream commits the patches apply to are in
[`config/upstream.lock`](../config/upstream.lock). See
[docs/versioning.md](../docs/versioning.md).

`scripts/build-kernel.sh` applies all of them automatically.
