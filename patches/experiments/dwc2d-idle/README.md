# Fewer dwc2d wakeups at idle (experiment `dwc2d-idle`)

Issue [#211](https://github.com/redox-rpi/redox-rpi/issues/211), for #126
(power). At idle, dwc2d is the main CPU load on the Pi 3B+ (cputop, Pi
test 239): 2.5 % CPU and about 77 wakeups/s.

- **base `0090`:**
  - The test request in `/tmp/dwc2d-cycle` (base `0033`) is looked for on
    every third rescan instead of every one. A path that doesn't exist is
    a round trip through the namespace manager and RedoxFS each time.
  - The network adapters are asked for their link every 5 s, and when one
    is new, instead of every second. On the LAN7800 that is two PHY reads,
    each with a 1 ms sleep.
  - `/tmp/dwc2d-wakeups`, every 10 s: the event loop's wakeups by source
    (`irq`, `scheme`, `time`, `hotplug`), the time spent on each (wall
    time, blocked calls included), and the parts of a hotplug rescan.

What was looked at and left alone:
- SOF interrupts are not enabled; only channel halt for the receive
  channel is.
- The LAN7800 receives only its own address and broadcast.
- The hub ports: asking each port takes 0.2 ms a rescan in QEMU. Asking
  the hubs' status change endpoint instead saved nothing there (QEMU's hub
  never NAKs it), so that was dropped.
- The keyboard is polled every 10 ms (about 46 wakeups/s in QEMU). The
  test Pi has none plugged in at idle, and a longer interval risks
  missing short key presses.
- `scheme` wakeups come from netstack, which reads every network scheme
  on each of its own timer ticks.

## Tests

- QEMU 2026-11-05, idle for 150 s with QEMU's keyboard and CDC Ethernet
  (`v0.3.0-dev-1385-g49f52b5+xd700` against the same build without the
  change):

  | | before | after |
  |---|---|---|
  | hotplug rescan, wall time | 44 ms/s | 21 ms/s |
  | of it the `/tmp/dwc2d-cycle` lookup | 44 ms/s | 20 ms/s |
  | port scan | 0.2 ms/s | 0.3 ms/s |
  | keyboard timer | 46/s | 48/s |
  | netstack requests | 7/s | 7/s |
  | receive interrupts | 0.6/s | 0.6/s |

  Keyboard test pass, `test-hotplug.sh` 15/15 (it uses
  `/tmp/dwc2d-cycle`).
- Pi: test-plan row (cputop at idle, before and after, and pi-test).
