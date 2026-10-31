# USB hub drop: diagnosis and recovery (experiment `usb-hub-drop`)

Issue [#151](https://github.com/redox-rpi/redox-rpi/issues/151). In Pi test
168, the Pi's USB hub with the network chip dropped off during a download
into the SD card (`dwc2d: hub 1 port 1: device at address 2 removed`,
`keyboard removed`). The system ran on, but dwc2d stopped: its
`/tmp/dwc2d-netstats` was no longer written, no other dwc2d line came, and
netstack hung waiting for it. A card-only write (167) and a network-only
download (141) were fine.

Applies after [`dwc2d-netstats`](../dwc2d-netstats/README.md):
`--experiment dwc2d-netstats --experiment usb-hub-drop`.

- **base `0083`:**
  - **No `sleep` in dwc2d.** The hotplug code waits 100 ms after a port
    change before it resets the port; nothing was printed after that point
    in test 168. A `nanosleep` in dwc2d already never returned in QEMU (50
    µs sleeps, while #104 was worked on). `delay()` yields the CPU until the
    time has passed.
  - **What the hub said:** a removal prints the port's status and change
    bits by name (connected, enabled, over-current, power; connection,
    enable, over-current changes) and the channel errors so far. Other
    changes (enable, suspend, over current) are printed and cleared.
  - **Recovery:** a port the hub disabled while its device stays connected
    (after a babble or an error) counts as unplugged, so the device is reset
    and found again. A port without power (switched off on over current) is
    switched on again. A device that fails to enumerate gets three more
    tries instead of waiting for a new plug-in. A disabled root port starts
    the enumeration over.
  - **Counters:** `/tmp/dwc2d-usbstats` after every rescan (once a second):
    channel errors by kind (transaction, babble, stall, data toggle, AHB,
    timeouts), the last error with its address and endpoint, hub port and
    root port events, recoveries, HPRT0 and GINTSTS.
    `/tmp/dwc2d-step` names the hotplug step dwc2d is in (`scan`,
    `attach hub A port P`, `probe address A`, `idle`), so a stuck dwc2d says
    where.
- `dwc2d-netstats` `0052` refreshed for today's main (the old one no longer
  applied).

## Tests

- QEMU: see below.
