# RFC 0005: Generic Redox extensions apart from the Pi parts

| | |
|---|---|
| Status | Draft |
| Created | 2026-11-28 |
| Related | [RFC 0002](0002-project-structure.md) (repository split, upstream rule) |

## Summary

About half of what we change in upstream Redox code has nothing to do with
the Raspberry Pi. It would work on Redox for x86_64 too. Today all of it
sits in one patch stack per component, mixed with the Pi drivers.

This RFC keeps the generic changes apart from the Pi-specific ones, in
two steps:

1. **A patch group inside `redox-rpi`:** `patches/generic/` is applied
   first and `patches/rpi/` on top of it. Both use the same pins.
2. **A nightly x86_64 QEMU build:** it builds only the generic set on
   upstream Redox and checks it.

A separate repository for the generic set is left as an open question.

## Motivation

- **Upstream:** a generic fix is easier to take when it is not mixed with
  board code and when it is shown to work on x86_64. The upstream rule in
  RFC 0002 already asks for small, clean commits; this RFC sorts them.
- **Other users:** someone who runs Redox on x86_64 or on another aarch64
  board can use the generic set without the Pi drivers.
- **Clear review:** a generic patch must not contain Pi code. A Pi patch
  may depend on a generic one, never the other way round.
- **Less drift:** when upstream fixes the same thing, the generic patch is
  easy to find and to drop.

## What we carry today

Counted on `main` (bc49b2be), 2026-11-28. A few borderline patches are
counted in one class.

Classes:

- **generic:** independent of the architecture.
- **aarch64:** ARM only, but not the Pi. Upstream supports aarch64, and
  QEMU `virt` can test it.
- **Pi:** BCM2835, DWC2, Pi firmware, Pi devicetree, U-Boot for the Pi.

### Default patches (138)

| Component | generic | aarch64 | Pi | Generic examples |
|---|---|---|---|---|
| base (76) | 28 | – | 48 | netstack (13: several adapters, TCP close and reset, multicast, ping), dhcpd (6), init, initnsmgr, usbhidd, inputd, getty, audiod, the driver-network link file, testnetd |
| kernel (26) | 14 | 7 | 5 | thread exit, blocking pipe write, scheduler, lock waits, scheme request cancel, memory faults that kill the process |
| relibc (21) | 14 | 7 | – | clock_settime, time zones and gmtime, futex, netdb, signals, getsockname, ld.so |
| redoxfs (5) | 5 | – | – | rename, write coalescing |
| bootloader (3) | 3 | – | – | only boot a RedoxFS with a kernel, no panic |
| netutils (3) | 3 | – | – | nc, ifconfig |
| u-boot (3) | – | – | 3 | Pi 4 GENET, lwip |
| fdt (1) | 1 | – | – | inherited interrupt parent |
| **Total** | **68** | **14** | **56** | |

The Pi patches in base are drivers:

- dwc2d: 29
- the SD controllers: 9
- rpi-firmwared: 4
- GPIO, I2C, SPI and PWM audio: 4
- the hardware RNG and the PCI opt-out: 2

The aarch64 patches are:

- in the kernel: the signal entry, cache and TLB maintenance, PL011;
- in relibc: FP registers across fork and signals, x18, the static
  TLSDESC fix.

### Experiments

About a third of the experiments are generic, for example:

- **sshd:**
  - procmgr, netstack and ptyd patches;
  - relibc setitimer, setgroups, poll and ld.so;
  - ion;
  - openssh.
- **ipv6:** netstack and relibc.
- **pkg-persist:** pkgutils.
- **Others:** the redoxfs experiments, timesync, timerd, homesync,
  serverfs, mdns.

Almost half are Pi experiments (vc4d, v3d, rpi-fb, sdhost, dwc2d, Pi 400,
HDMI audio). The rest are aarch64 experiments or image options.

### Programs in `tools/` (41)

- **generic (about 28):** timesync, timerd, zoneinfo, serverfs, homesync,
  pkg-persist, redox-fatfs2, redoxfs-file, mdnsd, mcast, net6, netdiag,
  cputop, audiotop, mmapstop, oomtest, chaos, polltest, ttyinfo, reboot,
  welcome, devshd, bugreport, and others.
- **Pi (about 9):** rpi-boot-stub, socdump, i2c, spi, and the VC4 and VPU
  probes.
- **aarch64 (2):** icachetest, sigregs.
- **Host tool:** log-triage.

## Proposal

### Stage 1: a patch group in `redox-rpi`

- **Layout:**
  - `patches/<component>/` becomes `patches/generic/<component>/` and
    `patches/rpi/<component>/`;
  - aarch64 patches go into `generic`, with `aarch64-` at the start of the
    name after the number, so a check can leave them out on x86_64.
- **Build:** `build-image.sh` applies `generic` first, then `rpi`.
  `config/upstream.lock` pins both, as today, and the build manifest
  records both.
- **Experiments:** `patches/experiments/<name>/config.toml` gets
  `class = "generic"` or `class = "rpi"`.
- **Tools:** generic programs go to `tools/generic/<name>/`, Pi programs
  to `tools/rpi/<name>/`, until the `tools` repository from RFC 0002
  exists. That repository then keeps the same split.
- **Rule:** a generic patch contains no board code. Where the boards
  differ, a generic patch reads configuration (for example
  `/etc/net/config.toml`) or an init unit uses `condition_board`. A Pi
  patch may build on a generic one.
- **Moving:** one pass sorts and renumbers the existing patches. Pi
  patches that build on generic ones (for example dwc2d on the
  driver-network link file) are rebased onto the generic stack. The image
  must stay byte-for-byte the same, apart from the manifest. A build
  compares this.
- **Check:** a script fails when a file under `patches/generic/` touches a
  Pi path or name (`bcm2835`, `rpi`, `dwc2`, `raspi`).

### Stage 2: a nightly x86_64 QEMU check

The build host is x86_64 with KVM, so this is cheap to run.

The nightly does:

1. Build upstream Redox for `x86_64-unknown-redox` with Redox's own server
   or desktop config, on the same `upstream.lock` pins.
2. Apply only `patches/generic/` (without the `aarch64-` patches) and add
   the generic tools.
3. Boot it in `qemu-system-x86_64` with KVM.
4. Run:
   - the relibc tests (as in the `relibc-tests` experiment);
   - timesync against a local NTP server;
   - an sshd login over user networking (once sshd is a default);
   - a few netstack checks (DHCP, TCP, ping).
5. Report the result next to the aarch64 nightly.

Notes:

- It runs nightly, not per pull request, because a full build takes about
  an hour.
- **Risk:** upstream x86_64 may not build cleanly on our pins. The first
  run will show this. Setting it up is about a day of work.
- A green x86_64 run is the best argument that a generic patch is really
  generic.

## How this fits the existing split

| Repository | Changes with this RFC |
|---|---|
| `redox-rpi` | patches and tools are split into `generic` and `rpi` |
| `apps` (2026-11-24) | none: it holds software people install; the generic set is part of the operating system |
| `tools` (RFC 0002, planned) | keeps the `generic`/`rpi` split |
| `drivers` (RFC 0002) | none: only Pi drivers; generic driver interfaces (the network link file, usbhidd) stay patches to base |
| `relibc` fork (RFC 0002, decision 3) | when the patches become commits: branch `generic` on upstream, branch `rpi` on top |
| `vc4d`, `rpi-open-firmware` | none |

## Not in this RFC

- No change to the release process or to what is in an image.
- No upstream posts. Contact with the Redox project is the maintainer's;
  the appendix is only a list.
- No x86_64 images for users. The x86_64 build is a check only.

## Open questions

1. **A separate repository.** Should the generic set move to its own
   repository, for example `redox-rpi/redox-extensions`? It would hold the
   patches per component, the generic tools as cookbook recipes, and CI
   for x86_64 and aarch64. `redox-rpi` would pin it in `config/repos.lock`
   (RFC 0002).
   - For: upstream and x86_64 users see a clean set, not a Pi repository.
   - Against: many generic fixes come from Pi work (a netstack fix that
     dwc2d needs). These would need two pull requests in two repositories.
   - Proposal: decide once stage 2 has run green for some weeks, or when
     upstream or x86_64 users show interest.
2. **The aarch64 class.** Should it be a third group (`patches/aarch64/`),
   with its own QEMU `virt` check, instead of a name prefix?
3. **The programs in `apps`.** Some generic programs (serverfs, homesync)
   are services people may install later. Should they stay in `tools` or
   go to `apps`?

## Steps

| Step | Content |
|---|---|
| 1 | Sort and renumber the patches into `generic` and `rpi`; `build-image.sh` applies both; the image is unchanged |
| 2 | `class` in the experiment configs; split `tools/` |
| 3 | The check script for Pi names in `patches/generic/` |
| 4 | The nightly x86_64 QEMU build and boot with the generic set |
| 5 | The checks in the x86_64 run: relibc tests, timesync, netstack, later sshd |

## Appendix: upstream candidates

This is a list for the maintainer, not a plan for posts. A candidate is
small, has an issue and a test, and fixes something that upstream has in
the same way.

- **kernel:**
  - blocking pipe write (0010);
  - thread exit (0008, 0013);
  - scheme request cancel on SIGKILL and Stop (0056, 0062);
  - memory faults that end the process, not the kernel (0058, 0060,
    0061);
  - the scheduler (0012).
- **relibc:**
  - clock_settime and settimeofday (0011);
  - time zones and gmtime (0012, 0026);
  - futex (0015);
  - EAI_NONAME (0019);
  - signals (0020–0022);
  - getsockname (0028);
  - from sshd: setitimer, setgroups, poll.
- **relibc aarch64:** FP and signal registers (0001, 0003, 0004, 0013) and
  the static TLSDESC fix (0025). These are not for x86_64, but upstream
  supports aarch64, so they are among the most useful.
- **base:**
  - netstack TCP fixes (0049, 0089, 0097–0099);
  - ping (0120);
  - multicast (0083, 0085);
  - dhcpd (0032, 0113, 0127);
  - init (0122), initnsmgr (0123), usbhidd (0124, 0125), inputd (0091),
    getty (0024), audiod (0128);
  - from sshd: procmgr and ptyd.
- **redoxfs:** rename (0036). Write coalescing (0037–0040) changes
  behaviour and needs a discussion first.
- **Small ones:** netutils (0001–0003), fdt (0001), bootloader (0002,
  0003).
- **Programs:** timesync (upstream has no NTP client in the image) and
  zoneinfo, as recipes.

Not candidates: dwc2d, the SD drivers, rpi-firmwared, GPIO, I2C, SPI, PWM
audio and the U-Boot patches. These are Pi drivers; upstream could take
them later as a whole.
