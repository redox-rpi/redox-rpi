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

## Results

**QEMU** (`raspi3b`, SD-card image `v0.3.0-dev-98-g9b91443+x9d2b` with
`ldso-diagnostics timesync timerd timerd-test timezone`, 2026-10-29):

| Command | Output |
|---|---|
| `date` / `date -u` | `Thu Oct 29 13:25:26 CET 2026` / `Thu Oct 29 12:25:27 UTC 2026` |
| `ls -l /etc/localtime` | `/etc/localtime -> /usr/share/zoneinfo/Europe/Berlin` |
| `date -d '@1782907200'` (1 July, 12:00 UTC) | `Wed Jul  1 14:00:00 CEST 2026` |
| `date -d '@1793260800'` (29 October, 08:00 UTC) | `Thu Oct 29 09:00:00 CET 2026` |
| `env TZ=UTC date` | `Thu Oct 29 12:25:34 UTC 2026` |
| `timerd` log | `clock set by timesync, 2026-10-29 13:23:07 Europe/Berlin`, timers on schedule |

relibc names the zone `Europe/Berlin` where Linux says `CET`/`CEST`
(`tm_zone` from chrono-tz). The summer time switch itself is covered by
`timerd`'s unit tests (`cargo test`): the skipped hour in March, the
repeated hour in October, and midnight right after the switch.

Not yet tested on a real Pi.
