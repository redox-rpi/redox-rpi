# Experiment: timerd, run commands at set times

Redox has no cron, and its `init` has no timers.
[`tools/timerd`](../../../tools/timerd) runs commands from timer files at
set times ([#22](https://github.com/redox-rpi/redox-rpi/issues/22)).

```bash
scripts/build-image.sh --experiment timesync --experiment timerd
```

The experiment has no patches, only a [`config.toml`](config.toml): the
`timerd` package, the service `70_timerd.service` and
`/etc/timer.d/README`. For two test timers add `--experiment timerd-test`.

## Timer files

One file per timer in `/etc/timer.d/<name>.toml` (or `/usr/lib/timer.d`;
a file in `/etc` replaces one of the same name there):

```toml
[timer]
description = "Nightly backup"
schedule = "30 3 * * *"   # minute hour day month weekday, in UTC
persistent = true         # run once at start if a run was missed

[service]
cmd = "backup"
args = ["--all"]
envs = { LANG = "C" }
```

- **Schedule:** the five cron fields with `*`, numbers, ranges `1-5`,
  steps `*/10` and lists `0,30`, or `@hourly`, `@daily`, `@weekly`,
  `@monthly`, `@yearly`. When both day fields are set, either one is enough,
  as in cron. Times are in UTC: Redox has no time zones set up yet.
- **Waits for the clock:** the Pi has no battery-backed clock. `timerd`
  starts its schedule only after `timesync` has the time from an NTP server
  (it writes `/tmp/timesync.synced`), at most five minutes. Then it goes on
  with the clock it has (saved or build time).
- **Catch up:** the time of each run is saved in `/scheme/data/timerd/<name>`
  (data partition of the network-boot SD card) or `/var/lib/timerd/<name>`.
  With `persistent = true`, a run that was missed while the Pi was off runs
  once at start.
- **Clock jumps:** a timer runs once if one of its times lies between two
  checks. If the clock jumps forward by hours, each timer runs once, not
  once per missed time; if it goes back, nothing runs twice.
- **Logs:** each start and exit code goes to the log, as `timerd: <name>: …`.
  A timer whose previous run has not ended is skipped.
- The directories are read again every 30 seconds: new, changed or deleted
  timer files need no restart.

```bash
timerd -l     # timers with schedule, next and last run
```

The cron parser is part of the tool, without a crate, so nothing new has
to build for Redox; its tests run with `cargo test` on the build host.

Before writing it, we checked what Redox plans: there are no timers in
`init`, and rustysd (a systemd-like service manager Redox uses for its
server demo) only starts services. Timers in `init` itself could be proposed
to Redox later, by a person.

## Results

**QEMU** (`raspi3b`, SD-card image `v0.3.0-dev-92-g2666483+x1044` with
`ldso-diagnostics timesync timerd timerd-test`, 2026-10-28):

1. With network: `timerd: clock set by timesync, 2026-10-28 20:03:50 UTC`,
   then `test-minute` ran `date` at 20:04, 20:05, 20:06 and 20:07, and
   `test-catchup` at 20:04 and 20:06, each with `exited with code 0`.
2. The same disk again after 8 minutes off: `test-catchup: starting uname
   (missed since 2026-10-28 20:06:00 UTC)` right after the clock was set;
   `test-minute` (not persistent) did not catch up. Then both ran on
   schedule again.
3. A fresh copy without network: `timerd: no time from timesync after
   300 s, going on with 2026-10-28 20:05:45 UTC` (the build time plus
   five minutes), then both timers ran on schedule.

Not yet tested on a real Pi ([test plan](../../../docs/test-plan.md)).
