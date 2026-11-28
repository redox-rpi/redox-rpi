# RFC 0004: Start services when they are needed

| | |
|---|---|
| Status | Draft |
| Created | 2026-11-27 |
| Issues | [#764](https://github.com/redox-rpi/redox-rpi/issues/764), [#765](https://github.com/redox-rpi/redox-rpi/issues/765), [#766](https://github.com/redox-rpi/redox-rpi/issues/766), [#767](https://github.com/redox-rpi/redox-rpi/issues/767), [#768](https://github.com/redox-rpi/redox-rpi/issues/768); related [#763](https://github.com/redox-rpi/redox-rpi/issues/763), [#622](https://github.com/redox-rpi/redox-rpi/issues/622) |
| Releases | v0.8.0 (dhcpd on link state), later steps open |

## Summary

A service runs only while something needs it. A trigger starts it, and it
ends or goes idle when that need is gone. Possible triggers are a device
that appears, a link that comes up, the first open of its scheme, a user,
or a timer. Whatever it set up while running (an address, a route, a
mode) it takes down again when the need goes. Polling is not a way to
wait for a need.

## Motivation

Today init starts every service in `/usr/lib/init.d` at boot, on every
board and in every image, whether or not the board has the hardware or
the user wants the function.

The trigger case is `dhcpd` on the Zero 2 W (Test 675):

- It starts at boot with no cable, watches `netcfg` and retries every 30 s.
- After the cable is pulled, the interface keeps the old address and
  default route (#763). The address was set up for a link that no longer
  exists, and nothing takes it down.

Why this matters:

- **Correct state:** what the system shows (address, route, console)
  should match the hardware. Today a service that started for a need
  outlives the need.
- **Memory:** the Zero 2 W has 512 MB. Each resident daemon costs about
  1.6–8 MB (table below).
- **Boot time:** each service start costs 0.06–0.2 s, and a few cost more
  ([boot-time.md](../boot-time.md)). The boot-time goal is next to no
  boot time.
- **Quiet boot, less to attack:** fewer lines in the log, and no network
  shell (`devshd`) listening where nobody asked for it.
- **Power and wake-ups:** a daemon that polls or feeds idle hardware wakes
  the CPU.

## The principle

1. **Need first.** Every service names what it needs to be useful: a
   device, a link, a user, a scheme being opened, a time. If that need
   cannot be named, the service does not start by default.
2. **Start on a trigger, not on boot.** Order of preference:
   1. not in the image at all (an experiment or image option);
   2. started by a trigger;
   3. resident but idle, blocked on an event (no timer, no polling), for
      small daemons whose start is slower than their wait;
   4. always active, only for what everything needs (init, logd,
      storage, input, USB, the serial console).
3. **Undo on the way out.** When the need goes (link down, device
   removed, idle), the service removes what it set up, then ends or goes
   back to idle. For dhcpd: release the lease and remove the address and
   the routes.
4. **Events come from the owner of the state.** The driver or daemon that
   knows the state publishes it: `netcfg` for links, `dwc2d` for USB
   devices, `rpi-fbd` for the monitor. Services wait on that; they do not
   ask again every N seconds.
5. **One place decides what runs.** Triggers are written in the init unit
   files, so `ls /usr/lib/init.d` still says what can run and why. Long
   term no daemon starts another on its own.
6. **The user can see it.** A service that is not running because its need
   is missing says so in one place (a status line per unit), not as a
   log line every 30 s.

## Mechanism (proposal)

Redox's `init` reads TOML units with `requires_weak`, `condition_board_not`
and `type = { scheme = … }`. Three additions, in this order:

1. **Conditions that init checks once at boot:** board (exists),
   devicetree node present, file present (for example
   `/etc/net/config.toml` names an interface). This is cheap, and it covers
   "not on this board" and "not configured".
2. **Event triggers:** a unit can say `start_on = "net.link_up:eth0"` and
   `stop_on = "net.link_down:eth0"`. Init gets the events from the scheme
   that owns the state (for links, a `netcfg` event file it can block on).
   Until init has this, a resident daemon that blocks on the same event is
   the allowed stopgap (level 3 above). dhcpd is already close to that.
3. **Scheme activation:** a unit with `type = { scheme = "audio" }` can be
   marked `on_demand`. Init then registers the scheme name and starts the
   daemon at the first open, then hands over the open. This needs a way to
   pass an open on in the scheme layer. It is the most work, and
   upstream Redox would have to want it. It is left for later.

Upstream: init, the unit format and dhcpd are upstream Redox components.
Steps 1 and 2 can start as our patches. Step 3 is a proposal for upstream,
to be discussed there by the maintainer.

## Today's services

Measured in QEMU raspi3b on 2026-11-27 with the main image plus relibc
0025/0026 (`v0.7.0-dev-67-gdd4f3bdc`): `ls /usr/lib/init.d`, `ps` after
login. `free` reports 113 MiB used of 904 MiB. Memory is `ps` PRIVATE per
process; part of it is the shared C library and Rust runtime, so the sum
overstates what removing a service would free. Boot times are from
[boot-time.md](../boot-time.md).

| Service | Started | Needed when | Trigger and stop | Memory | Proposal |
|---|---|---|---|---|---|
| `dhcpd` (`10_dhcpd`) | always, resident | an interface with `ipv4 = "dhcp"` (#622) has a link | start or work on link up; on link down release the lease and remove address and routes (#763) | 1.9 MB | **first step** |
| `timesync` (release image) | once at boot, then by timer | the network has an address | start after the first lease; exit after setting the clock; repeat through `timerd` | small, exits | with the dhcpd step |
| `netstack` | always | an interface exists, or a program uses a local socket | keep; start before the first interface or local socket | 6.9 MB | keep for now |
| `devshd` (`11_devshd`) | lab netboot image only (`config/raspi3bp.toml`); release images (`raspi3bp-sdcard.toml`, `--release-netboot`) already have none | lab tests | out of the image; installed on the card's data partition and started by a lab hook ([Lab tools](#lab-tools-outside-the-image)) | 5.2 MB | lab tools |
| `audiod`, `bcm2835-pwmaudio`, startup sound | always | a program opens `/scheme/audio`, or the startup sound is on | scheme activation; startup sound only if on | 3.6 + 1.7 MB; pwmaudio had 1.0 s CPU in the first minute in QEMU | check the idle CPU now; on demand later |
| `bcm2835-gpiod`, `-i2cd`, `-spid` (initfs) | always | a program or driver uses GPIO, I2C or SPI (the 3B+ power LED uses GPIO) | gpiod: keep (LED); i2cd and spid: scheme activation, or only with a devicetree overlay | 1.6 MB each | i2c and spi on demand |
| `vesad`, `fbbootlogd`, `fbcond`, `getty@2` + `login` | always | a monitor is connected | start on EDID or hotplug from `rpi-fbd` (experiment); on a headless board not at all | about 4 + 7.7 + 6.2 + 1.9 + 5.9 MB | later, with `rpi-fbd` hotplug |
| `getty` on serial + `login` | always | the serial console is the lab's way in | keep | 1.9 + 5.8 MB | keep |
| `sudo --daemon` | always | someone runs `sudo` | first `sudo` | 5.2 MB | low priority |
| `15_rpi-governor` | once, holds up boot (1.3 s in QEMU) | a governor is set in `/etc/rpi-firmware/governor` | `oneshot_async`, or a condition on the file | exits | boot time, not memory |
| `00_tmp` | once, 1.0 s in QEMU | always | already a one-off; make it an init built-in (boot-time experiment) | exits | boot time |
| `pcid-spawner` | not on the 3B+ (`condition_board_not`) | a PCI bus (Pi 4/400 PCIe) | already conditional | — | model for the others |
| `redoxfs data` (`60_data`) | always | a data partition exists | condition on the partition (today it fails with a red error line when there is none) | 34 MB on the root, small on data | condition |
| desktop (Orbital), `sshd`, `ha-bridge`, `mqtt` (experiments) | when the experiment is in | a user logs in to the desktop; the network is up; a broker is set | Orbital at graphical login; sshd on link up; ha-bridge by timer | — | follow the principle when they become default |

Always needed, out of scope: kernel, `init`, `procmgr`, `logd`, `randd`,
`ipcd`, `zerod`, `ramfs`, `rpi-firmwared`, `inputd`, `dwc2d`, the SD
driver, root `redoxfs`, `ptyd`.

What the numbers say:

- On the Zero (512 MB), a headless image without `devshd`, audio,
  I2C/SPI and the screen stack would save something like 30–45 MB of
  `ps` memory. That is about 6–9 % of RAM: worth having, but not the main
  reason.
- The main gains are correct state (#763), a quiet boot, no open shell
  in the release, and less to start. Boot time gains come mostly from
  the oneshot services that hold up the boot (governor, `/tmp`), not
  from the resident ones.

## Lab tools outside the image

The lab netboot image (`config/raspi3bp.toml`) differs from the release
netboot image (`--release-netboot`) by a few things only: `devshd`,
`socdump`, the data partition mount (`60_data`) and the startup-sound unit
that uses it. Non-release builds also get the test experiments
(`netstack-watch`, `oom-test`). The idea: the lab image becomes the release
image plus one small hook. The lab tools are installed once on the test
card's data partition and stay there until a new version is needed.

### Where they live

- The lab tools are packages in the lab package repository on the build
  host (experiment `pkg-repo`, published with `scripts/publish-pkg-repo.sh`).
  One package `redox-rpi-lab` depends on them: `devshd`, `socdump`,
  `oomtest`, more later.
- They are installed with `pkg-persist` (experiment `pkg-persist`) into
  `/scheme/data/pkg` on the data partition. That partition survives a reset
  and a netboot.
- At boot the lab hook does two things, both only in the lab image:
  - `pkg-persist` copies the files into the RAM root (`61_pkg-persist`, as
    today);
  - `62_lab.service` starts `devshd` if `/usr/bin/devshd` exists and its
    version matches. That is a boot condition from step 3
    ([#766](https://github.com/redox-rpi/redox-rpi/issues/766)).

### Version check

A lab tool is linked against the image's relibc and talks to the kernel
and the schemes. So what has to match is not the tool's own version but
the system ABI:

- The build writes `/etc/lab/abi` into the lab image: a short hash of the
  pinned kernel and relibc commits plus `patches/kernel` and
  `patches/relibc`.
- The lab packages are built with the same hash in their version
  (`<version>+abi<hash>`), and `pkg-persist` records it.
- The hook compares the two. If they differ, it starts nothing and writes
  one line: `lab: tools built for abi X, image has abi Y; reinstall`.
  `devshd` then does not answer, which is the signal for Pi Tests below.
- Linking `devshd` statically, as `sigregs` already is, takes relibc out
  of the dependency, but kernel and scheme changes still count. The ABI
  check stays either way.

### Replacing a version

`pkg-persist update` (or `install` after an ABI change) puts the new
version into `/scheme/data/pkg` and copies it into `/`. `pkg` replaces a
package only after its download is complete. If the install fails, the old
version stays on the partition, and the hook still refuses it while the
ABI differs. Nothing half-installed is started. The tools are replaced
only when the ABI hash or a lab package changes, not with every image.

### Installing over the serial console, with no person

Pi Tests already drives the release netboot image over the serial console
alone (T-SEC rows), so the pieces exist:

1. Stage the lab image and boot it. `pi-test.py` waits for
   `redox login:` on serial and tries devsh, as today.
2. If devsh does not answer within 30 s after the login prompt (new card,
   data partition reset, ABI changed), `pi-test.py --ensure-lab` logs in
   on the serial console. It runs `pkg-persist install redox-rpi-lab`,
   then `/usr/lib/lab/start` (the hook's start step), and waits for devsh.
   This takes about a minute, mostly the download over the LAN.
3. Then the row goes on through devsh as today. `pi-test.py` notes in its
   result whether it installed or found the tools.

What the serial console needs:

- the serial `getty` with a root login, which the image has today
  (`30_getty_debug`);
- network and the lab package repository on the build host's HTTP server
  (already there for `pkg-repo`);
- typing through the one serial reader that Pi Tests already uses (never
  a second `ttyUSB0` reader).

There is no file transfer over the serial line. Fallback without network,
if ever needed: a base64 archive typed into `base64 -d`. `devshd` is about
260 KB, so that is roughly 30 s at 115200 baud.

### Effect on the scripts and images

| Part | Change |
|---|---|
| `scripts/pi-test.py` | `--ensure-lab`: install over serial when devsh is missing (above); otherwise unchanged |
| `scripts/netboot.sh` | none (it stages files and does not use devsh) |
| lab image (`config/raspi3bp.toml`) | becomes `raspi3bp-release-netboot.toml` plus the data mount, `pkg-persist` and the lab hook; no `devshd` and no `socdump` packages |
| release images (`raspi3bp-sdcard.toml`, `--release-netboot`) | no change: no lab hook, so even a card with lab tools on its data partition starts nothing and nothing listens. A release-build check fails the build if `devshd` or the hook is in the image ([#765](https://github.com/redox-rpi/redox-rpi/issues/765)) |
| person tests, the Zero 2 W and Pi 400 cards | each card needs one install; the install is the same |

### What it gains

- **Size:** small. `devshd` is 258 KB, `socdump` 322 KB and `oomtest`
  390 KB, so under 1 MB of the image's 256 MiB file system. The
  netboot image does not get noticeably faster to load.
- **The real gain:** the lab image and the release image become the same
  except for the hook, so every lab row tests what users get. There is one
  image variant less to keep in step. Lab tools can change without a new
  image, as long as the ABI is the same.
- **Cost:** the first boot after an ABI change, or of a fresh card, takes
  about a minute more. A test that wipes the data partition reinstalls.
  `devshd` starts after the data partition and the network, a little later
  than now.

## Consoles: the screen console only with a monitor

Today every image starts two consoles:

- `30_getty_debug`: getty on `/scheme/debug`, the serial port;
- `getty@2`: getty on the screen console (`fbcond`).

Each starts a `login`. On a Zero 2 W with no monitor, `ps` shows both
pairs, and the screen one waits at a prompt that nobody can see. Together
with `vesad`, `fbbootlogd` and `fbcond` this is about 25 MB of `ps`
memory (table above).

### Proposal

- **Need:** the screen console is needed when a monitor is attached.
- **Detection at boot:**
  - Where `vc4d` runs, it reads the EDID over DDC and knows the hotplug
    state.
  - Elsewhere, `rpi-firmwared` asks the firmware for the display state
    (mailbox property `GET_EDID_BLOCK`; it fails when no display is
    attached).
  - The owner publishes the state as a file it can block on, for example
    `/scheme/display/connected` (principle 4).
- **Start:** `getty@2` and `login` start only when that file says
  connected. The unit uses a condition from step 3 now, and
  `start_on = "display.connected"` from step 5 later.
- **Hotplug:** a monitor plugged in later raises the event.
  - With `vc4d` this is the HPD interrupt.
  - The firmware path has no hotplug event. There, the boot check is all
    we get, unless the HPD pin is read through `bcm2835-gpiod` with an
    edge interrupt.
  - No polling in either case.
- **Unplug:** the session is kept. Ending a logged-in shell because a
  cable moved would lose work. The getty for an empty prompt ends.
- **Boot log on screen:** `fbbootlogd` follows the same rule. Kernel
  messages on serial are unchanged.
- **Person tests** use a monitor and a keyboard, so they get the console
  as today.

This refines step 6 (screen stack on monitor hotplug). The order:

1. The boot check through the firmware, for all boards.
2. Hotplug with `vc4d`.

Serial login in release images and default passwords are a separate
question: [RFC 0006](0006-console-login-release.md).

## Not in this RFC

- No new init system. This extends the units Redox already has.
- No change to how drivers find hardware: that stays with each driver.
- No per-user services. Starting at login is one more trigger, and it
  belongs with per-user schemes (RFC 0001, home-server storage).

## Steps

| Step | Content | Issue |
|---|---|---|
| 1 | dhcpd on link state: work starts on link up for interfaces in the config; on link down release the lease and remove address and routes; no retries while the link is down | [#764](https://github.com/redox-rpi/redox-rpi/issues/764), refs #763, #622 |
| 2 | release-build check (no `devshd`, no lab hook) and a user-guide note on how to get a development shell | [#765](https://github.com/redox-rpi/redox-rpi/issues/765) |
| 2b | lab tools outside the image: `redox-rpi-lab` package, ABI check, lab hook, `pi-test.py --ensure-lab` | [#768](https://github.com/redox-rpi/redox-rpi/issues/768), after #766 |
| 3 | init conditions at boot: devicetree node, file present; use them for `60_data`, the governor, i2c and spi | [#766](https://github.com/redox-rpi/redox-rpi/issues/766) |
| 4 | audio idle cost: why `bcm2835-pwmaudio` uses CPU with nothing playing; stop the stream when idle | [#767](https://github.com/redox-rpi/redox-rpi/issues/767) |
| 5 | init event triggers (`start_on`, `stop_on`) from `netcfg`; move dhcpd and timesync onto them | after step 1 |
| 6 | screen stack on monitor hotplug: first a boot check through the firmware, then hotplug with `vc4d` ([Consoles](#consoles-the-screen-console-only-with-a-monitor)) | later |
| 7 | scheme activation in init (audio, i2c, spi, sudo), proposed upstream | later |

## Open questions

- Should `netstack` itself wait for the first interface? Local sockets
  (devsh over loopback, IPC) suggest no.
- Where does the status of a unit live ("not running: no link")?
  `/scheme/init/...` or `ps`?
- For the screen stack: is a console without a monitor needed for
  NanoKVM and late hotplug, so that it must stay?
