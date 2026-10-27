# Experiment: dwc2d with interrupts

dwc2d waits for events instead of yielding in a busy loop: the controller
interrupt for received frames, requests on the network scheme, and a timer
for the keyboard (every 10 ms, plus key repeat).

```bash
scripts/build-image.sh --experiment dwc2-interrupts
```

Four patches:

- **kernel `0003`:** the BCM2835 interrupt controller only called kernel
  handlers; an interrupt without one was dropped and, being level
  triggered, fired again at once. Now the line is masked, the driver is
  woken through the irq scheme, and the line is unmasked when the driver
  writes its acknowledgement.
- **kernel `0004`:** every aarch64 irq chip marks all its lines as `used`
  at init, and the irq scheme read that flag as "reserved", so opening any
  line through `/scheme/irq/phandle-*` failed with `EEXIST`.
  Reservations now have their own flag.
- **kernel `0005`:** when dwc2d on another CPU clears its source between
  the interrupt and the kernel's read of the pending register, the kernel
  found nothing pending and logged two errors (34 times in one QEMU run).
  It now returns quietly.
- **base `0017`:** dwc2d opens its interrupt from the devicetree
  (`/scheme/irq/phandle-<intc>/1,9`) and enables only the channel-halted
  interrupt of the receive channel. All other transfers (sending, control
  requests, the keyboard) still wait for their channel by polling, which
  takes microseconds. Without an interrupt it falls back to polling every
  10 ms.

Like the other base experiments it carries patch number `0017`, so it
cannot be combined with `sdhost-driver` or `rpi-firmware` yet.

**Result:** not tested yet.
