# Time on the Pi

How the Pi gets its time, how to check and set the clock, time zones,
timers that run commands at set times, and a battery-backed clock module.
For people using the image; the [user guide](user-guide.md) covers the
basics of Redox first.

The Raspberry Pi has no clock that keeps running while it is off. Every
boot starts at 1 January 1970, and the image's tools move it to the right
time.

## What is where

Not everything here is in the stable image yet. Features stay on their own
branch until they passed a test on a real Pi.

| Feature | Where | State |
|---|---|---|
| Time from the network at boot (`timesync`) | stable image (experiment `timesync`) | in use since v0.2.0 |
| `timesync` waits for the network, retries | branch `timerd` | Pi tests #27, #41 passed; planned for v0.4.0 |
| Timers (`timerd`) | branch `timerd`, experiment `timerd` | Pi test #27 passed; planned for v0.4.0 ([#22](https://github.com/redox-rpi/redox-rpi/issues/22)) |
| Time zones | branch `timezone`, experiment `timezone` | QEMU only; planned for v0.5.0 ([#46](https://github.com/redox-rpi/redox-rpi/issues/46)) |
| Setting the clock by hand (`timesync -s`) | branch `clock-settime` | QEMU only; planned for v0.9.0 ([#51](https://github.com/redox-rpi/redox-rpi/issues/51)) |
| DS3231 clock module | branch `ds3231` | QEMU only; planned for v0.8.0 ([#39](https://github.com/redox-rpi/redox-rpi/issues/39)) |

## Checking the time

```sh
date                # local time (UTC without a time zone)
date -u             # UTC
uptime              # time since boot, e.g. "4m 30s"
timesync -n         # ask the time servers, do not change the clock
```

`timesync -n` prints for example
`timesync: 2026-10-29 14:53:55 UTC from pool.ntp.org (clock not set)`.
With a DS3231 module (branch `ds3231`) it also prints the module's time.

## How the clock is set at boot

The service `65_timesync` runs `timesync -d`:

1. **At once, without a network:** it sets the clock to the newest of
   - the image's build time (`BUILD=` in `/etc/redox-rpi-release`),
   - the time saved at the last run, in `/scheme/data/timesync.last`
     (network-boot SD card) or `/var/lib/timesync.last`,
   - the DS3231 module, if one is connected (branch `ds3231`).

   So the clock never goes back before the last boot, as with Linux'
   fake-hwclock.
2. **When the network is up:** it asks `pool.ntp.org`, then
   `time.cloudflare.com` (SNTP, UDP port 123), sets the clock and saves
   the time.
3. **Every hour:** it asks again and saves the time.

The boot log shows what happened:

```
timesync: clock set to 2026-10-29 11:05:15 UTC from the build time
timesync: clock set to 2026-10-29 13:26:04 UTC from pool.ntp.org
```

On branch `timerd`, `timesync` waits until DHCP has set a default route,
gives each DNS lookup 5 seconds, and tries again every 30 seconds until a
server answers:

```
timesync: no time server answered within a minute (last: no default route (DHCP not done))
timesync: trying again every 30 s
```

**Other time servers:** give them as arguments, for example
`timesync -n ptbtime1.ptb.de` to test one. The service's servers are set
in the image's configuration (`args` in `65_timesync.service`, see
[`patches/experiments/timesync`](../patches/experiments/timesync/config.toml)).

**No network at all:** the clock stays at the saved or build time and
keeps running from there. Set it by hand (next section) or connect a
DS3231 module.

## Setting the clock by hand

On branch `clock-settime` (with relibc patch `0008`), as root:

```sh
timesync -s '2026-10-29 14:05'        # UTC; seconds are optional
timesync -s '2026-10-29 14:05:30'
timesync -s '@1793282730'             # seconds since 1970
```

The time is in **UTC**, not local time. `timesync` prints
`timesync: clock set to 2026-10-29 14:05:00 UTC by hand` and saves the
time, so the next boot starts from there. A wrong format gives exit code 2.

In Ion an argument that starts with `@` must be quoted, as above.

`date -s` does not work on Redox: it says `setting the date is not
supported by Redox`. With patch `0008`, C and Rust programs can set the
clock with `clock_settime(CLOCK_REALTIME, …)` or `settimeofday()`; only
root may.

## Time zones

On branch `timezone` (experiment `timezone`) the image is set to
**Europe/Berlin**, with summer time:

```sh
date                         # Thu Oct 29 13:25:26 CET 2026
date -u                      # Thu Oct 29 12:25:27 UTC 2026
ls -l /etc/localtime         # /etc/localtime -> /usr/share/zoneinfo/Europe/Berlin
```

As on Linux, `/etc/localtime` is a link to the zone's file in
`/usr/share/zoneinfo`. The image has only two zones: `Europe/Berlin` and
`Etc/UTC`.

**Switching between them**, as root:

```sh
ln -sf /usr/share/zoneinfo/Etc/UTC /etc/localtime            # UTC
ln -sf /usr/share/zoneinfo/Europe/Berlin /etc/localtime      # back to Berlin
```

`date` follows at once. On the network-boot image the change is lost at
reboot (the root filesystem is in memory).

**For one command**, set `TZ`:

```sh
env TZ=UTC date
env TZ=CET-1CEST,M3.5.0,M10.5.0/3 date    # a POSIX rule: works in date
```

A zone name without a file in the image, such as `env TZ=Europe/London date`,
gives UTC without a warning. For another zone for one command, use a POSIX
rule as above, `NAMEoffsetDST,start,end`; the sign of the offset is the
opposite of the usual one: `CET-1` means UTC+1.

**Which programs follow the zone:** Rust programs such as `date` read the
zone file; C programs and `timerd` use relibc, which has all zones built in
and takes only the name from the link. The logs of `timesync` stay in UTC.

**Another zone in the image:** add its file to
[`tools/zoneinfo`](../tools/zoneinfo) and change the link in
[`patches/experiments/timezone/config.toml`](../patches/experiments/timezone/config.toml).
The stable image has no time zone and shows UTC.

## Timers: running commands at set times

Redox has no cron. `timerd` (branch `timerd`, experiment `timerd`) runs
commands from timer files at set times. One file per timer, in
`/etc/timer.d/<name>.toml`:

```toml
[timer]
description = "Nightly backup"
schedule = "30 3 * * *"   # minute hour day month weekday
persistent = true         # run after a boot if a run was missed

[service]
cmd = "backup"
args = ["--all"]
envs = { LANG = "C" }
```

| Field | Meaning |
|---|---|
| `schedule` | Five cron fields: minute (0-59), hour (0-23), day of month (1-31), month (1-12), weekday (0-7, 0 and 7 = Sunday). Each with `*`, a number, a range `1-5`, a step `*/10` or a list `0,30`. Or `@hourly`, `@daily`, `@weekly`, `@monthly`, `@yearly`. |
| `persistent` | `true`: a run missed while the Pi was off runs once after the next boot. Default `false`. |
| `utc` | `true`: the schedule is in UTC even if the image has a time zone (branch `timezone`). |
| `description` | Shown by `timerd -l`. |
| `cmd`, `args`, `envs` | The program, its arguments and extra environment variables. |

Examples of schedules:

| Schedule | Runs |
|---|---|
| `*/5 * * * *` | every 5 minutes |
| `0 * * * *` | every full hour |
| `30 3 * * *` | every day at 3:30 |
| `0 8 * * 1-5` | at 8:00 on working days |
| `0 0 1 * *` | at midnight on the first of each month |

**Creating a timer on the Pi** without an editor, as root:

```sh
echo -e '[timer]\nschedule = "*/2 * * * *"\n\n[service]\ncmd = "uptime"' > /etc/timer.d/uptime.toml
```

or with the editor: `kibi /etc/timer.d/uptime.toml`. `timerd` reads the
directory again every 30 seconds; a new, changed or deleted file needs no
restart. A file in `/etc/timer.d` replaces one of the same name in
`/usr/lib/timer.d`, where packages put theirs.

**Checking:**

```sh
timerd -l
```

```
uptime:  "*/2 * * * *", next 2026-10-29 16:16:00 Europe/Berlin, last 2026-10-29 16:14:00 Europe/Berlin
    uptime
```

A file with a mistake shows here with the error, for example
`uptime: error: TOML parse error at line 1, column 1`.

Each run is logged on the screen and the serial console:

```
timerd: uptime: starting uptime (scheduled)
timerd: uptime: exited with code 0 after 0.8 s
```

**After a boot** `timerd` waits until `timesync` has the time from the
network (at most five minutes), so timers do not run at the build time.
Without a network it goes on after five minutes with the clock it has.

**Things to know:**

- Commands run as root, without a shell. For pipes or `$(…)`, run Ion:
  `cmd = "ion"`, `args = ["-c", "uptime > /tmp/uptime.txt"]`.
- A timer whose last run has not finished is skipped.
- If the clock jumps forward by hours (the first NTP answer), each timer
  runs once, not once per missed time.
- With a time zone, a time in the hour skipped in March does not run; a
  time in the hour repeated in October runs once.
- On the network-boot image the root filesystem is in memory: files in
  `/etc/timer.d` are lost at reboot. Put lasting timers into the image's
  configuration (see the experiment
  [`timerd-test`](../patches/experiments/timerd-test/config.toml) for an
  example).

## DS3231 clock module

A DS3231 module keeps the time with a coin cell while the Pi is off, so
the clock is right at boot even without a network. On branch `ds3231`
(experiments `gpio`, `i2c` and `timesync`).

**Wiring** (Pi switched off), module to the Pi's header:

| Module | Pi pin |
|---|---|
| VCC | 1 (3.3 V) |
| SDA | 3 (GPIO 2) |
| SCL | 5 (GPIO 3) |
| GND | 6 (ground) |

At boot `timesync` reads the module (`/scheme/i2c/1/68`) and uses its time
if it is valid; after each answer from a time server it sets the module.

```
timesync: clock set to 2026-10-29 11:10:22 UTC from the RTC
timesync: RTC set
```

`timesync: not using the RTC: its oscillator stopped (battery?)` means the
module lost its time (empty or no coin cell); it is set again at the next
answer from a time server. Without a module `timesync` says nothing about
it. This has been tested in QEMU, not yet with a real module.

## Problems

| What you see | Why, and what to do |
|---|---|
| `date` shows the image's build day | No answer from a time server yet. Check the network (`ifconfig`, `ping -c 2 1.1.1.1`); `timesync -n` shows the error. |
| `no default route (DHCP not done)` | DHCP has not finished; `timesync` keeps trying every 30 s (branch `timerd`). |
| `timesync: cannot set the clock …` | Not root. |
| Timers do not run right after boot | `timerd` waits up to five minutes for `timesync`. `timerd -l` shows the next run. |
| `date` shows UTC | The image has no time zone (stable image), or `/etc/localtime` is missing. |
