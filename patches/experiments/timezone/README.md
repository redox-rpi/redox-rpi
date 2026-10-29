# Experiment: time zone

Issue [#46](https://github.com/redox-rpi/redox-rpi/issues/46), planned for
v0.5.0. Local time with summer time, here Europe/Berlin.

```bash
scripts/build-image.sh --experiment timesync --experiment timerd --experiment timezone
```

As on Linux, `/etc/localtime` is a symlink to the zone's file in
`/usr/share/zoneinfo`. The two kinds of programs on Redox use it
differently:

- **Rust programs** (chrono, for example uutils `date`) read the file the
  link points to.
- **relibc** (`localtime_r`, C programs, and `timerd`) has the tz database
  built in (`chrono-tz`) and only takes the zone's name from the link.

So the image needs both: the package `zoneinfo`
([`tools/zoneinfo`](../../../tools/zoneinfo)) with the zone files, and the
link, set in [`config.toml`](config.toml). To use another zone, add its
file to `tools/zoneinfo` and change the link.

`TZ` in the environment overrides the link for one program, for example
`env TZ=UTC date`. relibc understands only zone names there (`Europe/Berlin`),
chrono also POSIX rules (`CET-1CEST,M3.5.0,M10.5.0/3`).

`timerd` schedules use local time (see its README); `utc = true` in a
timer file keeps one in UTC. Logs from `timesync` stay in UTC.
