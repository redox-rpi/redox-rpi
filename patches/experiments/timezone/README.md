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

In this first build relibc named the zone `Europe/Berlin` where Linux says
`CET`/`CEST` (`tm_zone`). The summer time switch itself is covered by
`timerd`'s unit tests (`cargo test`): the skipped hour in March, the
repeated hour in October, and midnight right after the switch.

**CET/CEST in relibc:** relibc patch
[`0012`](../../relibc/0012-time-tm_zone-is-the-zone-abbreviation-CET-CEST-not-t.patch)
puts the abbreviation into `tm_zone` (and so into `strftime("%Z")`), as
other systems do. `timerd -t <seconds>` shows a time as relibc's
`localtime_r` sees it.

**QEMU**, network boot, build `v0.3.0-dev-337-ge7d1cdd+x0598`
(experiments `ldso-diagnostics timesync timerd timezone`, 2026-10-30), the
switches of 2026 (25 October 01:00 UTC, 29 March 01:00 UTC):

| UTC | `date -d '@…'` (zone file, chrono) | `timerd -t …` (relibc) |
|---|---|---|
| 25 Oct 00:30:00 | `Sun Oct 25 02:30:00 CEST 2026` | `2026-10-25 02:30:00 CEST (+02:00)` |
| 25 Oct 00:59:59 | `Sun Oct 25 02:59:59 CEST 2026` | `2026-10-25 02:59:59 CEST (+02:00)` |
| 25 Oct 01:00:00 | `Sun Oct 25 02:00:00 CET 2026` | `2026-10-25 02:00:00 CET (+01:00)` |
| 25 Oct 01:30:00 | `Sun Oct 25 02:30:00 CET 2026` | `2026-10-25 02:30:00 CET (+01:00)` |
| 29 Mar 00:59:59 | `Sun Mar 29 01:59:59 CET 2026` | `2026-03-29 01:59:59 CET (+01:00)` |
| 29 Mar 01:00:00 | `Sun Mar 29 03:00:00 CEST 2026` | `2026-03-29 03:00:00 CEST (+02:00)` |

Also: `date` `Fri Oct 30 09:25:11 CET 2026` against `date -u` `08:25:13
UTC`; `env TZ=UTC timerd -t 1792890000` gives `01:00:00 UTC (+00:00)`; the
boot log says `timerd: clock set by timesync, 2026-10-30 09:24:42 CET`.
In Ion, `@…` must be quoted (`'@1792890000'`), else Ion reads it as an
array.

**Pi test 95** (real Pi 3B+, `v0.3.0-dev-481-gcfa7d09+x9d2b`, 2026-10-30):
the same outputs for `date`, `date -d` and `timerd -t` at both switches,
`date` in CET next to `date -u`, and `TZ=UTC` gives UTC. 20 minutes of the
`timerd-test` timers without a stall (slowest run 5.3 s; the old image of
test 49 once stalled for 438 s).
