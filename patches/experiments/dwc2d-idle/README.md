# Fewer dwc2d wakeups at idle (experiment `dwc2d-idle`)

Issue: #126 (power). At idle, dwc2d is the main CPU load on the Pi 3B+
(cputop, row 239): 2.5 % and about 77 wakeups/s.

- **base `0090`:** counts the event loop's wakeups by source (`irq`,
  `scheme`, `time`, `hotplug`) and the time spent on each, written to
  `/tmp/dwc2d-wakeups` every 10 s.

## Tests

- QEMU: see below.
