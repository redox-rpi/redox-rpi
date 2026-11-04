# Watchdog for a stuck netstack (experiment `netstack-watch`)

Issue: [#92](https://github.com/redox-rpi/redox-rpi/issues/92). Test 211:
during a download netstack stopped for good. It showed as *blocked* with
little CPU, and `cat /scheme/netcfg/ifaces/eth0/addr/list` never returned.

netstack runs on one thread, so one call that never returns stops the whole
stack. **Base `0001`** has the main loop record what it is doing: which event
it handles, and whether it is writing to or reading from the network device.
A second thread writes to the serial console (`/scheme/debug`, opened before
netstack enters its null namespace) when that has taken more than 5 s, and
again every 30 s:

```
netstack-watch: main loop stuck in write to the network device for 5 s
netstack-watch: main loop back after 12 s
```

Nothing is printed while netstack works normally. This is a diagnostic,
not a fix.

Build: `--experiment netstack-watch`, for the Pi with `--experiment
dwc2d-netstats` too, to see whether `dwc2d` still runs.

## Tests

- QEMU `raspi3b` (`v0.4.0-dev-279-gc08c4839+x42c6`): normal boot, DHCP and
  `ping` work, no `netstack-watch` line. Then `kill -STOP` on `dwc2d` and a
  `ping`: within 5 s `netstack-watch: main loop stuck in read from the
  network device`, repeated every 30 s, and netstack answered nothing else:
  the same picture as test 211 on the Pi. So a `dwc2d` that stops answering
  is enough to give #92's symptoms; the device's `O_NONBLOCK` doesn't help
  then.
- Found on the way: `kill -CONT` on the stopped `dwc2d` crashed it
  (`UNHANDLED EXCEPTION … dwc2d`), and netstack stayed in that read even
  after `dwc2d` was gone.
