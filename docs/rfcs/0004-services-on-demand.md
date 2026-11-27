# RFC 0004: Start services when they are needed

| | |
|---|---|
| Status | Draft |
| Created | 2026-11-27 |
| Issues | [#764](https://github.com/redox-rpi/redox-rpi/issues/764), [#765](https://github.com/redox-rpi/redox-rpi/issues/765), [#766](https://github.com/redox-rpi/redox-rpi/issues/766), [#767](https://github.com/redox-rpi/redox-rpi/issues/767); related [#763](https://github.com/redox-rpi/redox-rpi/issues/763), [#622](https://github.com/redox-rpi/redox-rpi/issues/622) |
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
| `devshd` (`11_devshd`) | always, TCP port 2323, no login | development only | not in release images; in lab images explicitly | 5.2 MB | **release: off** |
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

## Not in this RFC

- No new init system. This extends the units Redox already has.
- No change to how drivers find hardware: that stays with each driver.
- No per-user services. Starting at login is one more trigger, and it
  belongs with per-user schemes (RFC 0001, home-server storage).

## Steps

| Step | Content | Issue |
|---|---|---|
| 1 | dhcpd on link state: work starts on link up for interfaces in the config; on link down release the lease and remove address and routes; no retries while the link is down | [#764](https://github.com/redox-rpi/redox-rpi/issues/764), refs #763, #622 |
| 2 | `devshd` not in release images; lab images keep it explicitly | [#765](https://github.com/redox-rpi/redox-rpi/issues/765) |
| 3 | init conditions at boot: devicetree node, file present; use them for `60_data`, the governor, i2c and spi | [#766](https://github.com/redox-rpi/redox-rpi/issues/766) |
| 4 | audio idle cost: why `bcm2835-pwmaudio` uses CPU with nothing playing; stop the stream when idle | [#767](https://github.com/redox-rpi/redox-rpi/issues/767) |
| 5 | init event triggers (`start_on`, `stop_on`) from `netcfg`; move dhcpd and timesync onto them | after step 1 |
| 6 | screen stack on monitor hotplug (with `rpi-fbd`) | later |
| 7 | scheme activation in init (audio, i2c, spi, sudo), proposed upstream | later |

## Open questions

- Should `netstack` itself wait for the first interface? Local sockets
  (devsh over loopback, IPC) suggest no.
- Where does the status of a unit live ("not running: no link")?
  `/scheme/init/...` or `ps`?
- For the screen stack: is a console without a monitor needed for
  NanoKVM and late hotplug, so that it must stay?
