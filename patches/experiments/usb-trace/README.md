# Experiment usb-trace

Test only (`test-only`). For #749: after a USB mouse was unplugged and
plugged in again on the Pi 400, the built-in keyboard's usbhidd stopped
getting reports.

`base/0001`: xhcid logs one `usb-trace:` line every 10 s (while events
come) with:

- `events`: transfer events by slot, endpoint ID and completion code
  (`s2e3c36:1` is one event on slot 2, endpoint ID 3, completion code
  36);
- `registered` / `matched`: transfer waiters registered and answered per
  ring (port and endpoint);
- `dead`: waiters on a ring that is gone, answered with an unrelated event;
- `unmatched`: transfer events with no waiter (kept for a late waiter, or
  lost);
- `pending`: waiters still waiting per ring; `early`: kept events.

If the keyboard's ring shows `pending 1` and no events, the device or hub
stopped answering; if it shows no `registered`, usbhidd or xhcid's worker
stopped reading.

`base/0002`: a usbhubd trigger that simulates an unplug, so a replug test
needs no person:

    echo "1.1 disable" > /tmp/usbhubd-cycle

clears PORT_ENABLE on hub port 1.1 once; the poll loop then resets the
port, detaches the device (its driver process ends) and attaches it again,
as after a replug. `echo "1.1 power" > /tmp/usbhubd-cycle` clears
PORT_POWER for 3 s instead, which is closer to an unplug but cuts every
port if the hub switches power ganged. At start usbhubd logs
`usb-trace: hub characteristics … (power switching ganged|per port|none)`.

`base/0003` (needs PR #750's pi400 base 0026): a second line every 10 s,
`usb-trace: resources commands N cmd_ring I/LEN cycle C workers W handles H
ports P`: commands sent, the command ring's enqueue index and cycle state,
blocking-read workers started, open scheme handles and attached ports. A
resource used up per replug shows as a count that grows each round.

`base/0004`: `echo 1000 > /tmp/xhcid-noop` makes xhcid send 1000 No Op
commands one after the other, with a progress line every 100
(`usb-trace: no-op N done, cmd_ring I cycle C`). It wraps the command
ring several times with no device involved. If the progress lines stop,
a command did not complete.

Build with `--experiment pi400 ... --experiment usb-trace` (after pi400).
