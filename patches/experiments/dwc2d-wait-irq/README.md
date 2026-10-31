# No busy-waiting during USB transfers (experiment `dwc2d-wait-irq`)

Issue: [#104](https://github.com/redox-rpi/redox-rpi/issues/104). `dwc2d`
already uses the DWC2's buffer-DMA mode: the controller moves the data. But
`Dwc2::transfer_on` started a transfer and then spun until the channel
halted. Network receive waits for the interrupt (base `0022`), but every sent
frame, every 64 KiB USB stick chunk and every control request spun, so a
stick write kept a core busy.

- **base `0053`:** `transfer_on` spins for 50 µs, about a full-size
  network frame or a control request, and then sleeps with a nap that
  doubles from 50 µs to 1 ms until the channel halts. Waiting for the
  channel's interrupt instead would need the interrupt line shared with
  the event loop; that is a later step if the naps cost speed.
- Applies on `usb-storage-more` (hotplug base `0031`–`0033`, experiments
  `usb-storage`, `usb-storage-more`): `--experiment usb-storage
  --experiment usb-storage-more --experiment dwc2d-wait-irq`.
- `scripts/measure-usb-cpu.sh [image]` (QEMU): dwc2d's CPU time (ACTTIME in
  `/scheme/sys/context`) and the speed for 30 s idle, a 64 MiB stick read, a
  32 MiB stick write and a 16 MiB TCP send to the host.

## Tests

- QEMU: see below.
