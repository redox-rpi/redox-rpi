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

Build with `--experiment pi400 ... --experiment usb-trace` (after pi400).
