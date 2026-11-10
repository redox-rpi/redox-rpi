# dwc2d-watch: what a stuck dwc2d main loop is doing (#92)

For [#92](https://github.com/redox-rpi/redox-rpi/issues/92). In QEMU,
netstack stalls 6–11 s while it waits for a read from `dwc2d`, and `dwc2d`
then uses almost no CPU, so it is waiting for something too. This watch
says what.

- **base `0001`:** `dwc2d` gets `src/watch.rs`, the same idea as experiment
  `netstack-watch`. The main loop records its stage: idle (waiting for
  events), interrupt, scheme requests, timer, stats timer, hotplug, hotplug
  scan, probe, and file write to `/tmp` (the diagnosis files of
  `dwc2d-netstats` and `usb-hub-drop`). A thread writes to `/scheme/debug`:
  - `dwc2d-watch: main loop stuck in <stage> for N s` after 5 s, again
    every 30 s, then `dwc2d-watch: main loop back after N s`;
  - every 60 s a heartbeat: `dwc2d-watch: alive at N s, <stage> for N s,
    events: N interrupt, N scheme, N other`.
- **How to read it:** if netstack is stuck in a read while `dwc2d` shows
  idle and its scheme count doesn't move, netstack's request never woke
  `dwc2d`. A stuck file write points at the disk.
- **Needs** `dwc2d-netstats usb-hub-drop dwc2d-restart`, given before it on
  the command line, since its patch goes on top of theirs.
