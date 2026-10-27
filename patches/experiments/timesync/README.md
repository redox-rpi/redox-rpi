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

**Result:** see below once tested.
