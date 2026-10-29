# dwc2d with interrupts (was experiment `dwc2-interrupts`)

**Default since 2026-10-28:** base `0022` and kernel `0004`–`0006` (moved to `patches/base/` and `patches/kernel/`) are now default patches and in every
image. `--experiment dwc2-interrupts` is still accepted and does nothing, so older
commands keep working. This page stays as the driver's documentation.

dwc2d waits for events instead of yielding in a busy loop: the controller
interrupt for received frames, requests on the network scheme, and a timer
for the keyboard (every 10 ms, plus key repeat).

The base patch applies after the RTL8152 driver (base `0020`), which
changes the same file.

Four patches:

- **kernel `0004`:** the BCM2835 interrupt controller only called kernel
  handlers; an interrupt without one was dropped and, being level
  triggered, fired again at once. Now the line is masked, the driver is
  woken through the irq scheme, and the line is unmasked when the driver
  writes its acknowledgement.
- **kernel `0005`:** every aarch64 irq chip marks all its lines as `used`
  at init, and the irq scheme read that flag as "reserved", so opening any
  line through `/scheme/irq/phandle-*` failed with `EEXIST`.
  Reservations now have their own flag.
- **kernel `0006`:** when dwc2d on another CPU clears its source between
  the interrupt and the kernel's read of the pending registers, the kernel
  found nothing pending and logged errors (34 times in one QEMU run). It
  now returns quietly. Also drops a debug message that `irq_enable` printed
  for every acknowledged interrupt.
- **base `0022`:** dwc2d opens its interrupt from the devicetree
  (`/scheme/irq/phandle-<intc>/1,9`) and enables only the channel-halted
  interrupt of the receive channel (channel 1, used by CDC Ethernet,
  LAN7800 and RTL8152). All other transfers (sending, control requests,
  the keyboard) still wait for their channel by polling, which takes
  microseconds. Without an interrupt it falls back to polling every 10 ms.

**Result in QEMU (2026-10-27, `v0.1.0-alpha-37-g13c3da6+dwc2-interrupts`,
before the rebase onto `rtl8152`):** `dwc2d` opens
`/scheme/irq/phandle-1/1,9`, the network (CDC Ethernet) and the USB
keyboard work (`scripts/test-keyboard.sh` passes), no "unexpected irq"
messages. Compared with the polling driver, both measured with `ps` about
80 s after boot, after 500 lines of output through devshd:

| | interrupts | polling |
|---|---|---|
| CPU time of `dwc2d` (user + system) | 0.95 s + 0.50 s | 4 min 21 s |
| 500 lines through devshd | 2 s | 26 s |

The remaining CPU time is mostly the keyboard: 100 polls per second,
each a split transaction that spins until the hub answers.

**Result on a real Pi 3B+ (2026-10-28, test #5,
`v0.3.0-dev-52-g7df70f9+xcb30` with `timesync`, `rtl8152`, `pi-clean-boot`):**
pass. `dwc2d` waits for the interrupt through the LAN7800, the
`scripts/pi-test.py` standard checks pass (0 of 300 starts failed), 2000
lines through devshd take 2 s, and the USB keyboard works on the HDMI
console. With nothing to do, `dwc2d` uses about 5 % of one core (2.85 s
in 62 s) instead of a full core; most of it is the keyboard polling
([test report](../../../docs/test-reports.md)).
