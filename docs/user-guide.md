# User guide

For people who know macOS, Windows or a Linux shell, but not Redox. It
covers the redox-rpi image on a Raspberry Pi 3 Model B+ (or in QEMU).
To get the image onto an SD card and boot it first, see
[getting started](getting-started.md). Every command here was tried on the stable image
`v0.3.0-dev-15-g24fbb96+x9f76` in QEMU (2026-10-29), except the keys of the
full-screen programs `kibi` and `less`, which come from their
documentation. The [USB devices](#usb-devices) section was checked on
today's main in QEMU and against the Pi tests (2026-11-01). Things that
are missing or behave differently are listed at the end.

Redox looks like a Unix system: there is a shell, `/home`, `ls`, `cat`
and most of the usual tools. Two things are different: the shell is
**Ion**, not bash, and devices and services are **schemes** under
`/scheme`.

## Logging in

At `redox login:` type a user name and press Enter.

| User | Password | Note |
|---|---|---|
| `root` | `password` | administrator |
| `user` | none | normal user |

Change a password with `passwd`. Log out with `exit`.

Where to type:

- **Keyboard and screen** on the Pi. A USB keyboard works whether it is
  plugged in before or after power-on (see [USB devices](#usb-devices)).
  US layout only.
- **Serial console** (pins 8 and 10 of the Pi header, 115200 baud), for
  example `screen /dev/ttyUSB0 115200` on Linux or macOS.
- **Over the network**, only with the network-boot image: `devshd` on port
  2323, for example `nc <pi-address> 2323` or
  `socat -,raw,echo=0 tcp:<pi-address>:2323`. It is not encrypted: use it
  only in your home network. There is no SSH yet.

## Moving around

Most commands are the same as on Linux or macOS (they come from
[uutils](https://github.com/uutils/coreutils), a Rust version of the GNU
tools):

```sh
pwd                  # where am I? (/home/root after login as root)
ls                   # list files; ls -l for details
cd /etc              # change directory; cd alone goes home
cat /etc/redox-rpi-release   # which image is this?
less /etc/redox-rpi-manifest # page through a file, q quits
mkdir demo; touch demo/a.txt; cp demo/a.txt demo/b.txt
mv demo/b.txt demo/c.txt; rm -r demo
df -h                # disk space
free -h              # memory
ps                   # processes; kill <pid> ends one
uptime
uname -a
```

There is no `dir`-style Windows shell and no Finder: everything is typed.

**Editing a file:** `kibi <file>` (a small editor; Ctrl-S saves, Ctrl-Q
quits, Ctrl-F searches). `nano` and `vim` are not in the image.

**Help:** `man <command>` for Redox' own tools, `<command> --help` for
most others.

## The Ion shell (instead of bash)

Ion looks like bash for simple commands, pipes and redirections:

```sh
ls /etc | wc -l
echo text > /tmp/a.txt
cat /tmp/a.txt >> /tmp/b.txt
true && echo yes
false || echo no
```

Differences you will meet:

| bash | Ion |
|---|---|
| `name=Pi` | `let name = Pi` |
| `export X=1` | `export X=1` (the same) |
| `echo "$name"` | `echo $name` |
| `$(date)` | `$(date)` (the same) |
| `if [ 1 = 1 ]; then echo yes; fi` | `if test 1 = 1; echo yes; end` |
| `for f in a b; do echo $f; done` | `for f in a b; echo $f; end` |
| `X=1 command` | `env X=1 command` |
| `~/.bashrc` | `~/.config/ion/initrc` (the folder `~/.config/ion` exists) |

`@` starts an array in Ion. An argument that begins with `@`, such as
`date -d @1782907200`, must be quoted: `date -d '@1782907200'`. Unquoted,
Ion stops the whole line with "Variable … does not exist".

`bash` and `sh` scripts do not run: there is no bash in the image.

## Files and schemes

The root filesystem (`/`) is RedoxFS on the SD card (SD-card image) or in
memory (network-boot image, lost at reboot). Besides files, Redox has
**schemes**: services that look like directories under `/scheme`. A
scheme is roughly what `/dev`, `/proc` and `/sys` are on Linux, but every
driver and service gets one.

```sh
ls /scheme                        # all schemes
cat /scheme/sys/uname             # kernel information
cat /scheme/rpi-firmware/temperature    # SoC temperature, in m°C
ls /scheme/disk.mmc               # the SD card and its partitions
```

| Path | What |
|---|---|
| `/scheme/disk.mmc` | SD card |
| `/scheme/data` | data partition of the network-boot SD card (keeps files across reboots) |
| `/scheme/netcfg` | network settings (see below) |
| `/scheme/rpi-firmware` | board, memory, temperature, clocks |
| `/scheme/sys` | kernel: `uname`, `log`, `kstop` (stop or reset) |
| `/scheme/null`, `/scheme/zero`, `/scheme/rand` | like `/dev/null`, `/dev/zero`, `/dev/urandom` |

Redox does not follow a symlink from the filesystem into a scheme, so
always use the `/scheme/...` path itself.

## Network

The Pi gets its address by DHCP at boot (Ethernet only; no Wi-Fi).

```sh
ifconfig                                  # address of eth0
cat /scheme/netcfg/ifaces/eth0/addr/list  # the same, raw
cat /scheme/netcfg/route/list             # routes; "default via" is the router
cat /scheme/netcfg/resolv/nameserver      # DNS server from DHCP
ping -c 2 192.168.178.1                   # Ctrl-C stops ping without -c
dns example.com                           # look up a name
```

`curl` and `wget` are not in the image.

## USB devices

USB on the Pi is run by one driver, `dwc2d`. It looks at every port once
a second, so devices can be plugged in and out at any time, also through a
USB hub. These work today:

| Device | What happens |
|---|---|
| USB keyboard | types on the screen console, US layout; plug it in at any time |
| Built-in Ethernet (LAN7800 on the 3B+) | the network at boot |
| USB Ethernet adapter with an RTL8152 or RTL8153 (e.g. Zero 2 W Ethernet HATs, the Anker 565 hub) | used as soon as it has a link; the network moves to it and keeps its address |
| USB hubs | devices behind them work like the others |

When several network adapters are plugged in, the network uses an RTL815x
with a cable first, then the built-in LAN7800, then an RTL815x without a
cable. The network keeps the same MAC address and DHCP lease when it
moves.

To see what `dwc2d` found, look at its lines on the screen or the serial
console at boot and whenever something is plugged in or out:

```
dwc2d: hub 1 port 2: device connected
dwc2d:   address 3: 0525:a4a2, USB 2.00, class 0x02, Full speed
dwc2d: keyboard at address 2
dwc2d: network on the adapter at address 3
```

The network adapter in use also shows in `ls /scheme`, as
`network.usb_lan78xx`, `network.usb_rtl8152` or, in QEMU,
`network.usb_ecm`.

**Not in the image yet**, each in a pull request with a test on the Pi
still to do: USB sticks and card readers
([#25](https://github.com/redox-rpi/redox-rpi/issues/25),
[#52](https://github.com/redox-rpi/redox-rpi/issues/52)), USB serial
adapters for Zigbee or Thread sticks and Arduinos
([#50](https://github.com/redox-rpi/redox-rpi/issues/50)), mice
([#48](https://github.com/redox-rpi/redox-rpi/issues/48)), and `lsusb`
([#35](https://github.com/redox-rpi/redox-rpi/issues/35)). Other USB
devices (sound, cameras, Wi-Fi sticks, printers) are not supported.

## Time and time zones

The Pi has no battery-backed clock. At boot `timesync` sets the clock to
the last saved time (or the image's build time), then from
`pool.ntp.org` once the network is up, and again every hour.

```sh
date              # current time, in UTC
date -u           # UTC explicitly
timesync -n       # ask the time server, do not change the clock
```

Time zones are not set up in the stable image: `date` shows UTC. For one
command a POSIX rule works, for example Germany with summer time:

```sh
env TZ=CET-1CEST,M3.5.0,M10.5.0/3 date
```

Zone names such as `Europe/Berlin` and a system-wide setting
(`/etc/localtime`) come with
[#46](https://github.com/redox-rpi/redox-rpi/issues/46) (planned for
v0.5.0). `date -s` cannot set the clock on Redox; `timesync -s` can
([#51](https://github.com/redox-rpi/redox-rpi/issues/51)). All of it, with
examples: [time.md](time.md).

## Timers (`timerd`)

Redox has no cron. `timerd`
([#22](https://github.com/redox-rpi/redox-rpi/issues/22), planned for
v0.4.0, not in the stable image yet) runs commands at set times from files
in `/etc/timer.d/`:

```toml
# /etc/timer.d/backup.toml
[timer]
schedule = "30 3 * * *"   # minute hour day month weekday, like cron
persistent = true         # run after a boot if a run was missed

[service]
cmd = "backup"
args = ["--all"]
```

`timerd -l` lists the timers with their next and last run. Schedules,
examples and what to watch out for: [time.md](time.md#timers-running-commands-at-set-times).

## Smart home (MQTT)

`mqtt-pub` and `mqtt-sub` send and receive MQTT messages, and
`ha-bridge` shows the Pi's temperature, clock, memory and uptime in Home
Assistant ([#49](https://github.com/redox-rpi/redox-rpi/issues/49), planned
for v0.8.0, not in the stable image yet). How to set them up:
[smarthome.md](smarthome.md).

## Installing programs (`pkg`)

`pkg` installs prebuilt packages from the Redox package server
(`static.redox-os.org`); it needs the network and root.

```sh
pkg list            # installed packages
pkg info kibi       # details of a package
pkg search <name>   # search (lists more than the matches, see below)
pkg install <name>
pkg remove <name>
pkg update          # update all installed packages
```

Many prebuilt aarch64 programs do not start yet (see
[known-issues.md](known-issues.md)), so a package that installs may still
not run.

## Rebooting and power-off

```sh
reboot                            # as root; restarts the Pi
echo reset > /scheme/sys/kstop    # the same, if reboot itself fails
```

There is no real power-off yet. `shutdown` stops the system but cannot
cut the power (`PSCI SYSTEM_OFF failed`); wait until the screen or serial
console shows `kstop`, then pull the power. A proper `poweroff` is
[#7](https://github.com/redox-rpi/redox-rpi/issues/7).

## Where the logs are

There are no log files yet. Boot messages and the output of services
(`timesync`, `dhcpd`, `devshd`, …) appear on the **screen and the serial
console**, mixed with the login prompt. To keep them, record the serial
console on another computer, for example with
`screen -L /dev/ttyUSB0 115200`.

`dmesg` exists but fails, and `/var/log` is empty.

## What is missing or different

Found while checking this guide on the stable image (QEMU, 2026-10-29):

| What | State |
|---|---|
| `dmesg` | fails: `/scheme/sys/log: Bad file number` |
| Log files | none; `/var/log` is empty |
| Power-off | `shutdown` halts without power-off ([#7](https://github.com/redox-rpi/redox-rpi/issues/7)) |
| Time zones | UTC only in the stable image ([#46](https://github.com/redox-rpi/redox-rpi/issues/46)) |
| Timers | `timerd` not in the stable image yet ([#22](https://github.com/redox-rpi/redox-rpi/issues/22)) |
| SSH | none; `devshd` (unencrypted) in the network-boot image only |
| Editors | `kibi` only; no `nano`, `vim` |
| Shells | Ion only; no `bash`, `sh` |
| Web tools | no `curl`, `wget` |
| `pkg search` | lists many packages that do not match |
| `sudo --help` | not an option: `sudo` runs its first argument as a command |
| `/etc/hostname` | has no line end, so `cat` runs into the next prompt |
| Keyboard | US layout only |
| USB | keyboard, Ethernet adapters and hubs only; no sticks, serial adapters or mice yet ([USB devices](#usb-devices)) |
