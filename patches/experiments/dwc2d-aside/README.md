# dwc2d waits for no other daemon in its main loop (experiment `dwc2d-aside`)

Issues [#92](https://github.com/redox-rpi/redox-rpi/issues/92) and
[#217](https://github.com/redox-rpi/redox-rpi/issues/217). A fix candidate,
not a proven fix: the stall has not been caught with `dwc2d-watch` yet.

**What the stalls show.** In Pi test 316, netstack hung for good in a read
from dwc2d (`netstack-watch: main loop stuck in read from the network
device`), once 5 s after the login prompt with no load and once 5 s after
four `dd bs=16M` started. netstack opens the adapter non-blocking, so the
read itself never waits for a frame; it waits only while dwc2d does not
take the request. dwc2d's hotplug timer also calls `scheme.tick()` once a
second, so a lost scheme event would cost at most a second. A read stuck
for minutes therefore means dwc2d's main loop itself is blocked, or never
runs again.

**Where the main loop can block.** Its USB waits all have timeouts, and the
kernel's IRQ read does not block. What remains are calls to other daemons:

- Mouse and keyboard events go to inputd (`ProducerHandle::write_event`)
  from the poll in the main loop. inputd's own main loop calls the active
  display driver synchronously for VT events (`call_wo` in
  `send_vt_event_to_active_display`), so a display driver that does not
  answer stops inputd, inputd stops the next input event, and that stops
  dwc2d and with it the network. #316 had vc4d and Orbital and the
  NanoKVM's absolute mouse on dwc2d.
- Once a second the hotplug handler reads `/tmp/dwc2d-cycle` and looks
  for `/tmp/dwc2d-input-log`: a file system that hangs stops dwc2d.

**What it changes (base 0001).** Input events go to inputd from a thread of
their own through a queue of 256 events; when inputd does not take them,
further events are dropped and counted (`dwc2d: inputd does not take input
events, N dropped so far`, at powers of two). The two test files are looked
at by another thread once a second; the main loop takes what it found. The
main loop then calls only the kernel, the USB controller, the timer and
logd (`println!`).

**What to look for in a test.** No change in normal use: mouse and keyboard
work, `echo restart > /tmp/dwc2d-cycle` and `touch /tmp/dwc2d-input-log`
still work (within a second). If the network still stalls with this
experiment, the cause is not one of these calls; if a `dropped` line comes
instead of a stall, inputd was the link.

The patch is made with one line of context, so it also applies after
`dwc2d-netstats`, `usb-hub-drop`, `dwc2d-restart` and `dwc2d-watch`.

## QEMU (2026-11-10)

`inputd-stop.sh`: boot with the USB keyboard and USB network, `kill -STOP`
inputd, type 200 keys through the QEMU monitor, then `ping -c 3 10.0.2.2`.

- **Without `dwc2d-aside`** (row 319's image, with `dwc2d-watch`): the
  network stalls for good 5 s after the first key, with the same line as Pi
  test 316, `netstack-watch: main loop stuck in read from the network
  device`, and `dwc2d-watch: main loop stuck in timer` (the keyboard poll,
  waiting in the write to inputd). Ping fails.
- **With `dwc2d-aside`** (`v0.5.0-dev-50-g845466be+x3676`): ping 3 of 3,
  no `stuck` line; `dwc2d: inputd does not take input events, N dropped so
  far` up to 1024.

So a stopped inputd is enough to give #316's picture, and the experiment
keeps the network going. Why inputd stopped on the Pi is still open: its
main loop waits for the display driver (vc4d in #316) on every VT event.
