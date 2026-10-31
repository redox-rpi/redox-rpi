# Sent network frames wait for the interrupt at once (experiment `dwc2d-tx-irq`)

Issue: [#104](https://github.com/redox-rpi/redox-rpi/issues/104), on top of
[`dwc2d-wait-irq`](../dwc2d-wait-irq/README.md). With `dwc2d-wait-irq`, a
transfer spins for 50 µs before it waits for the channel's interrupt. A sent
network frame is done within that time, so it always spun: on the Pi a 16 MiB
TCP send kept dwc2d busy about half the time, the same with and without
`dwc2d-wait-irq` (test #144).

- **base `0072`:** `Dwc2::send_frame` for the sent frames of the LAN7800,
  RTL8152/RTL8153 and CDC Ethernet adapters: no spin, it waits for the
  interrupt at once. Everything else keeps the 50 µs spin.
- Build: `--experiment usb-storage --experiment usb-storage-more
  --experiment dwc2d-wait-irq --experiment dwc2d-tx-irq`, plus
  `--experiment tcp-send` for the sender.

## Tests

- QEMU: see below.
