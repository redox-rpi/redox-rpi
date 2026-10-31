# Time

User docs, with examples: [docs/time.md](../time.md).

## State

- The Pi has no battery-backed clock. `tools/timesync` (experiment
  `timesync`, in v0.2.0) sets the time over NTP at boot; without a network
  it falls back to the last saved time or the build time. Tested on the
  Pi 3B+ and the Zero 2 W.

## In progress

- [#22](https://github.com/redox-rpi/redox-rpi/issues/22) `timerd`: run
  commands at set times from cron-format timer files, with catch-up after a
  restart; waits for `timesync`. Branch `timerd`, experiments `timerd` and
  `timerd-test` ([README](../../patches/experiments/timerd/README.md)).
  The same branch makes `timesync` wait for the network (default route,
  DNS with a timeout) and retry until a server answers. Pi tests #27
  (`timerd`) and #41 (`timesync` with late DHCP) passed; pull request for
  main open (v0.4.0).
- [#51](https://github.com/redox-rpi/redox-rpi/issues/51) `clock_settime`
  / `settimeofday` in relibc (patch `0011`) and `timesync -s` to set the
  clock by hand. Branch `clock-settime` (based on `timerd`); Pi test #66
  passed.

## Next

- Move `timesync` from an experiment into the default images.

## Later

- An I2C clock module (DS3231) for boots without a network.

## Timers for other services

Once `timerd` is in an image (experiment `timerd`, branch `timerd` until
it is merged), a service or a package gets periodic jobs by shipping a
timer file. No API, no restart:

- **A package** installs `/usr/lib/timer.d/<name>.toml`. Use the package
  or service name, so names do not clash.
- **An image config** adds the file with `[[files]]`, as the experiment
  configs do.
- **The user** puts a file of the same name in `/etc/timer.d/` to change
  it; a file there always wins over `/usr/lib/timer.d`. To switch a timer
  off, give that copy a schedule that never comes, such as `0 0 30 2 *`
  (February 30th).

`timerd` reads both directories every 30 seconds. Example, a daily record
of the free space on the data partition:

```toml
# /usr/lib/timer.d/df-daily.toml
[timer]
description = "Daily free space on the data partition"
schedule = "0 4 * * *"    # 04:00 UTC
persistent = true         # run after a boot if 04:00 was missed

[service]
cmd = "ion"
args = ["-c", "df >> /scheme/data/df.log"]
```

Things to know:

- **UTC:** schedules are in UTC; Redox has no time zones yet.
- **One run at a time:** a run is skipped while the previous one of the
  same timer still runs.
- **Clock:** the first run waits for `timesync` (at most five minutes),
  so jobs see the right time.
- **Shell features** (pipes, `>>`) need `ion -c "…"`, as above. `cmd` is
  started directly, without a shell.
- **Output:** standard output and the exit code go to the system log
  (`timerd: <name>: …`). A job that needs its own log writes it itself.
- **No triggers:** there are no event or boot-relative triggers
  (`OnBootSec` in systemd), only cron schedules. Ask in
  [#22](https://github.com/redox-rpi/redox-rpi/issues/22) if a service
  needs them.
