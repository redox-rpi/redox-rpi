# Restart a stuck USB controller (experiment `dwc2d-restart`)

Issue [#151](https://github.com/redox-rpi/redox-rpi/issues/151). In Pi test
189 (the long download into the SD card, with the scheduler fix), run 5
stopped at 320 MiB: first `Dropped outboud packet (failed to write to
network file)`, then `Timeout("channel halt")` on every request to hub 1
and hub 2, 542 lines, for good. No device was reported removed. dwc2d spun
on CPU 0 through one-second timeouts, and the network was gone. That image
did not have `usb-hub-drop`, but its recovery works from the hub port
status, which could no longer be read either.

Applies after [`usb-hub-drop`](../usb-hub-drop/README.md):
`--experiment dwc2d-netstats --experiment usb-hub-drop --experiment
dwc2d-restart`.

- **base `0084`:**
  - A channel whose transfer timed out was only told to stop, so the next
    transfer could start on a channel still busy. Now dwc2d waits until it
    halts; channels that do not halt are counted (`halt_failures` in
    `/tmp/dwc2d-usbstats`).
  - After four control requests in a row time out, the controller counts as
    stuck: dwc2d prints its registers (`GINTSTS`, `HPRT0`, `HFNUM`, the TX
    FIFO and queue space, and every channel's `HCCHAR`/`HCINT`/`HCTSIZ`) and
    the errors so far, forgets every device, resets and sets up the
    controller as at start, and the next rescan enumerates the devices
    again; the network comes back on its adapter with the same address.
  - `echo restart > /tmp/dwc2d-cycle` does the same, for tests.
  - A rescan stops asking hub ports once the controller is stuck, instead
    of one second per port.
  - After Pi test 190: the restart resets the core even when its AHB master
    does not get idle (`GRSTCTL` 0x40000000, a DMA request pending, two
    channels that never halted), and waits for idle after the reset, as
    Linux does. The reset switches the root port's power off on the Pi
    (`HPRT0` 0x400 after the restart, nothing found), so the restart
    switches it on again. When the devices are not back after a restart,
    dwc2d tries again every 10 s, up to 5 restarts in a row.

## Tests

- QEMU 2026-11-01, `v0.3.0-dev-974-g3cf2086+x809a` (with `netget`,
  `timesync` and `sched-vtime-clamp`, branch `t151-restart-test`): three
  `restart` requests in a row, each time the controller is reset, the hub,
  keyboard and network card are found again and devsh answers; keyboard
  test pass, `test-hotplug.sh` 15/15 without a false restart. QEMU first
  aborted on the restart (`usb_ep_get: dev != NULL`): its DWC2 goes on
  retrying a NAKed poll after the channel is halted, so the devices'
  channels are now halted as on unplugging before the reset.
- Pi test 190: the forced restart left the root port unpowered (nothing
  enumerated again); in a long download the controller got stuck and the
  restart failed with `Timeout("AHB idle")`. Fixed as above.
- Pi: see the test plan (after #190).
