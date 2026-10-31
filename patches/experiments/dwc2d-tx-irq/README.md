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

- QEMU 2026-10-31, both images from b2dd152 with `tcp-send`, 2 MiB over
  TCP twice: wait-irq `+x98e9` 0.16/0.14 MB/s, dwc2d CPU +11.61/+12.66 s;
  tx-irq `+x3388` 0.16/0.17 MB/s, +10.78/+10.23 s (about 13% less, same
  speed). tx-irq: storage 18/18, hotplug 15/15, keyboard pass.
- Pi 2026-10-31 (test 152, the tx-irq row), 16 MiB twice: wait-irq 7.28/8.30
  MB/s, dwc2d ACTTIME +1.03/+1.14 s; tx-irq 7.12/7.94 MB/s, +1.22/+1.10 s.
  **No gain on the Pi**; the QEMU saving does not carry over. Not added to
  `dwc2d-wait-irq` (#139).
