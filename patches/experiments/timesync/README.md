# Experiment: timesync, the clock from an NTP server

The Raspberry Pi has no real-time clock, so Redox starts at 1970 and files
are dated January 1st. [`tools/timesync`](../../../tools/timesync) asks an
NTP server (SNTP) for the time once the network is up and sets the clock.
It writes the boot time, in nanoseconds since 1970, to
`/scheme/sys/update_time_offset`, the same interface `rtcd` uses on PCs;
relibc has no `clock_settime` yet.

```bash
scripts/build-image.sh --experiment timesync
```

The experiment has no patches, only a [`config.toml`](config.toml): it adds
the `timesync` package and the service `20_timesync.service`, which starts
after `10_net.target`. It tries `pool.ntp.org` and `time.cloudflare.com`
for up to a minute, because DHCP may still be running.

By hand:

```bash
timesync -n              # print the time from the server, do not set it
timesync                 # set the clock (root)
timesync 192.168.178.1   # another server, here a router with NTP
date
```

**Result in QEMU** (`raspi3b` with `--usb-net`, build
`v0.1.0-alpha-24-g2e33285+ldso-diagnostics+timesync`, 2026-10-27): during
boot `timesync: clock set to 2026-10-27 08:06:46 UTC from pool.ntp.org`;
`date` then printed the same time as the build host. Log lines written
before the clock was set still carry 1970 timestamps. Not yet tested on a
real Pi ([test plan](../../../docs/test-plan.md), test 3).
