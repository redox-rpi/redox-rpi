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

Every 60 s it also writes a heartbeat:

```
netstack-watch: alive at 120 s, idle for 0 s, 3456 events (2900 network), 3100 frames in
```

If the heartbeats stop, the watchdog thread itself isn't running. If they go
on with `idle` while the counts stay the same, the main loop is waiting for
events that never come. This is a diagnostic, not a fix.

Build: `--experiment netstack-watch`; for the Pi with `dwc2d-netstats`,
`usb-hub-drop`, `dwc2d-restart` (is `dwc2d` still running?) and `timesync`
(HTTPS needs the right date).

## Tests

- QEMU `raspi3b` (`v0.4.0-dev-279-gc08c4839+x42c6`): normal boot, DHCP and
  `ping` work, no `netstack-watch` line. Then `kill -STOP` on `dwc2d` and a
  `ping`: within 5 s `netstack-watch: main loop stuck in read from the
  network device`, repeated every 30 s, and netstack answered nothing else:
  the same picture as test 211 on the Pi. So a `dwc2d` that stops answering
  is enough to give #92's symptoms; the device's `O_NONBLOCK` doesn't help
  then.
- Pi test 213 (first version, without the heartbeat): the network went
  28 s into `pkg install vim`, with **no** `netstack-watch` line in 4 min.
  `dwc2d` kept receiving (rx 1464 → 1775 in 49 s) while netstack sent 3
  frames; no USB error, `dwc2d-step` idle. So the main loop wasn't stuck in
  a device call: either it waited for events (idle, not reported then) or
  the watchdog thread didn't run either. The heartbeat tells which.
- Found on the way: `kill -CONT` on the stopped `dwc2d` crashed it
  (`UNHANDLED EXCEPTION … dwc2d`), and netstack stayed in that read even
  after `dwc2d` was gone.
