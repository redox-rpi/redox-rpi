# Experiment: timesync, the clock from an NTP server and the saved time

The Raspberry Pi has no real-time clock, so Redox starts at 1970 and files
are dated January 1st. [`tools/timesync`](../../../tools/timesync) asks an
NTP server (SNTP) for the time once the network is up and sets the clock.
It writes the boot time, in nanoseconds since 1970, to
`/scheme/sys/update_time_offset`, the same interface `rtcd` uses on PCs;
relibc has no `clock_settime` yet.

```bash
scripts/build-image.sh --experiment timesync
```

Without a network the clock still moves forward, like `fake-hwclock` on
Linux: at start `timesync` sets it to the newest of
- the image's build time (`BUILD=` in `/etc/redox-rpi-release`) and
- the time saved by an earlier run, in `/scheme/data/timesync.last` (the
  data partition of the network-boot SD card) or else
  `/var/lib/timesync.last` (SD-card image, where the root filesystem is
  kept).

The clock is never set back by this step. After an NTP answer, and every
hour with `-d`, it saves the time again. So after a boot without a network
the clock is at least where the last run left it, and never before the
image was built.

The experiment has no patches, only a [`config.toml`](config.toml): it adds
the `timesync` package and the service `65_timesync.service`, which runs
`timesync -d`. Named 65, it starts after `60_data.service` has mounted the
data partition (network-boot image). It does not wait for `10_net.target`:
it sets the saved time at once, waits until DHCP has set a default route
(`/scheme/netcfg/route/list`), then tries `pool.ntp.org` and
`time.cloudflare.com`. If no server answers within a minute it tries again
every 30 seconds until one does, then every hour. After the first answer
it writes `/tmp/timesync.synced`, which `timerd` waits for.

Redox' DNS lookup (relibc) waits for its answer without a timeout. On the
real Pi (test #27) `timesync` hung there when it asked before DHCP was
done. So it waits for the route first and runs each lookup in its own
thread with a 5-second timeout.

By hand:

```bash
timesync -n              # print the time from the server, do not set it
timesync                 # set the clock (root) and save it
timesync -d              # keep running: resync and save every hour
timesync 192.168.178.1   # another server, here a router with NTP
date
cat /var/lib/timesync.last /scheme/data/timesync.last
```

**Result in QEMU** (`raspi3b`, SD-card image, 2026-10-27):

- First version (build `v0.1.0-alpha-24-g2e33285+ldso-diagnostics+timesync`,
  with `--usb-net`): `timesync: clock set to 2026-10-27 08:06:46 UTC from
  pool.ntp.org`; `date` then printed the same time as the build host.
- With build time and saved time (build
  `v0.1.0-alpha-31-g87477b6+ldso-diagnostics+timesync`, built 08:46:44):
  1. With network: `clock set to 2026-10-27 08:46:44 UTC from the build
     time`, then `clock set to 2026-10-27 08:48:42 UTC from pool.ntp.org`.
     `/var/lib/timesync.last` held `1793090922` (08:48:42), `timesync -d`
     kept running, `date` matched the build host.
  2. The same disk again, without network: `clock set to 2026-10-27
     08:48:42 UTC from the saved time`.
  3. A fresh copy of the image, without network: `clock set to 2026-10-27
     08:46:44 UTC from the build time`.

The service starts after the login prompt appears, so log lines from early
boot still carry 1970 timestamps.

**Result on the real Pi** (2026-10-27/28, [test reports](../../../docs/test-reports.md)):
part of v0.2.0. NTP time matches the build host and the daemon writes
`/scheme/data/timesync.last` (Pi 3B+, netboot). On the v0.3.0-rc1 SD card
the boot set the build time, then the saved time, then the time from
`pool.ntp.org`. Also works on the Zero 2 W through an RTL8152 adapter.

**Late DHCP** (branch `timerd`, build `v0.3.0-dev-95-g0d47c4f+x43ba`,
QEMU with experiment [`late-dhcp`](../late-dhcp/README.md), which starts
`dhcpd` 90 seconds late, 2026-10-29): `no time server answered within a
minute (last: no default route (DHCP not done))`, `trying again every 30 s`,
then `clock set to 2026-10-29 11:02:00 UTC from pool.ntp.org`, and
`timerd: clock set by timesync` a second later.

## Setting the clock by hand (branch `clock-settime`)

Redox' `date` cannot set the clock, so `timesync` does:

```sh
timesync -s '2026-10-29 14:05'       # UTC; seconds optional
timesync -s '@1793282730'            # seconds since 1970 (quote the @ in Ion)
```

It sets the clock with `clock_settime`, which relibc patch `0011` adds
([#51](https://github.com/redox-rpi/redox-rpi/issues/51)), and saves the
time like an NTP answer. With a relibc without the patch (ENOSYS) it writes
`/scheme/sys/update_time_offset` itself, as before.

QEMU (build `v0.3.0-dev-98-g723b343+x7f63`): `timesync -s "2030-01-02 03:04:05"`
→ `date -u` shows 2030 and a new file gets that time; a later `timesync`
sets the clock back from NTP through the same call and saves it; a bad
argument gives exit code 2. The built `libc.so` exports `clock_settime`
and `settimeofday`. `date -s` still says "setting the date is not supported
by Redox" (uutils turns it off for Redox).
