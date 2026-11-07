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

netstack re-arms its timer at least every 500 ms, and each timer event also
polls the device, so the main loop never waits long. When no timer event has
come for 10 s and the loop is not stuck in a call, the thread writes that,
again every 10 min, and once more when timer events come back:

```
netstack-watch: no timer event for 10 s, idle for 10 s, 3456 events (2900 network), 3100 frames in
netstack-watch: timer events again after 42 s
```

That is a main loop waiting for events that never come: the case Pi test 213
could not show (see below). Every 10 min it also writes a heartbeat:

```
netstack-watch: alive at 600 s, idle for 0 s, 3456 events (2900 network), 3100 frames in
```

If the heartbeats stop, the watchdog thread itself isn't running. This is a
diagnostic, not a fix.

**On by default in test images** (`config/test-experiments.conf`): every
`scripts/build-image.sh` build gets it, so stable, nightly and Pi test-row
images have it until #92 and #217 are caught. Releases (`--release`) never
do: the `test-only` file here makes `--release` refuse it, and
`scripts/publish-pkg-repo.sh` refuses to publish from a build that has it.
`--no-test-experiments` leaves it out, for benchmark and A/B rows. It does
not go with `ipv6` (`conflicts`), which rewrites the same receive loop; with
`ipv6` the build leaves it out and says so.

By hand: `--experiment netstack-watch`; for the Pi with `dwc2d-netstats`,
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
  the watchdog thread didn't run either. The heartbeat tells which, and
  the timer alarm (2026-11-07) reports the first case on its own: the
  first version only reported a call that took over 5 s, never a loop
  that waits.
- Found on the way: `kill -CONT` on the stopped `dwc2d` crashed it
  (`UNHANDLED EXCEPTION … dwc2d`), and netstack stayed in that read even
  after `dwc2d` was gone.
- Pi tests 263 (40 chaos rounds) and 264 (10 warm resets), 60 s
  heartbeat: every heartbeat said `idle for 0 s`, no other line; neither
  bug came back.
- QEMU `raspi3b`, timer alarm and 10 min heartbeat, as a default test
  experiment (`v0.4.0-dev-625-g5a33b666+x2d42`, stable options
  `timesync rtl8152`): 11 min with ping at the start and end gave one
  heartbeat (`alive at 600 s, idle for 0 s, 1274 events`, about 2 per
  second: the 500 ms timer) and no alarm. `kill -STOP` on `dwc2d` and a
  ping: `stuck in read from the network device` after 5 and 35 s, and no
  timer alarm while that is reported. The alarm itself could not be
  triggered in QEMU: the timer comes from the kernel.
