# Fewer dwc2d wakeups at idle (experiment `dwc2d-idle`)

Issue [#211](https://github.com/redox-rpi/redox-rpi/issues/211), for #126
(power). At idle, dwc2d is the main CPU load on the Pi 3B+ (cputop, Pi
test 239): 2.5 % CPU and about 77 wakeups/s.

- **base `0093`** (`0090` in Pi rows 242 and 243):
  - (Until base `0087`: the test request in `/tmp/dwc2d-cycle` was looked
    for on every third rescan only. Since `0087` a thread of its own looks
    for it, outside the event loop, so this part is gone.)
  - The network adapters are asked for their link every 5 s, and when one
    is new, instead of every second. On the LAN7800 that is two PHY reads,
    each with a 1 ms sleep.
  - `/tmp/dwc2d-wakeups`, every 10 s: the event loop's wakeups by source
    (`irq`, `scheme`, `time`, `hotplug`), the time spent on each (wall
    time, blocked calls included), and the parts of a hotplug rescan.
- A keyboard poll on the controller interrupt was tried and dropped
  (Pi row 243, see below).

What was looked at and left alone:
- SOF interrupts are not enabled; only channel halt for the receive
  channel is.
- The LAN7800 receives only its own address and broadcast.
- The hub ports: asking each port takes 0.2 ms a rescan in QEMU. Asking
  the hubs' status change endpoint instead saved nothing there (QEMU's hub
  never NAKs it), so that was dropped.
- The keyboard is polled every 10 ms (about 46 wakeups/s in QEMU). A
  longer interval risks missing short key presses. On the Pi this turned
  out to be the main cost (see below): the test Pi has a low-speed
  keyboard (04d9:0006) behind a hub, also when the NanoKVM is unplugged.
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
- Pi row 242, 2026-11-05 (`v0.3.0-dev-1399-g0b9cd6c+xf5e2` before,
  `+xceb2` after, both with cputop), 5 minutes idle:

  | | before | after |
  |---|---|---|
  | dwc2d CPU | 2.5–2.6 % | 2.5 % |
  | dwc2d wakeups | 76–78/s | 75–76/s |
  | netstack wakeups | 26–29/s | 26–29/s |
  | irq line 50 (USB) | 7/s | 7/s |

  `/tmp/dwc2d-wakeups` after 300 s: irq 7.2/s, scheme 19.1/s, **time
  50.0/s with 6558 ms busy** (22 ms/s, about 2.2 % of a core), hotplug
  1.0/s with 559 ms busy (cycle request 79 ms, scan 445 ms, links 26 ms).
  So on the Pi the hotplug rescan was never the cost; the keyboard poll
  timer is, at about 130 µs per poll. pi-test passes on both;
  `pi-hotplug-test.sh` cycled the port twice, and its check of cycle 2
  came too early (fixed in the script: it now waits for the power-off
  line).
- With the keyboard poll on the interrupt (then base `0091`, dropped), QEMU 2026-11-05 (`v0.3.0-dev-1402-g36a22d8+xd700`):
  keyboard test pass, `test-hotplug.sh` 15/15 including typing on the
  replugged keyboard. In QEMU, dwc2d goes from 1.0 to 1.7 % CPU: QEMU's
  keyboard answers at once, so the extra interrupt wakeup per poll costs
  more than the spinning it saves. On the Pi each poll spun for two
  microframes.
- Pi row 243, 2026-11-05, with the keyboard poll on the interrupt
  (`v0.3.0-dev-1409-gb6b5aef+xceb2`, then base `0091`, commit 36a22d8):
  **fail, and dropped.** dwc2d's CPU went down from 2.5 to 1.2 %, but its
  wakeups went up from 77 to 240–480/s and irq line 50 from 7 to
  170–410/s: after the start split the hub answers NYET to most complete
  splits, so a poll took about 7 interrupts. 619 of 25984 polls never
  halted (stopped after 50 ms), and dwc2d hung after the second power-on
  in the hotplug test (no line after `cycle: … power on`, network gone).
  Split transactions need answers within microframes; from a user-space
  interrupt handler that is too slow (the Raspberry Pi Linux driver does
  this part in a FIQ for the same reason). The poll stays blocking; a
  cheaper keyboard poll needs another idea (for example a longer poll
  interval while no key is pressed).
