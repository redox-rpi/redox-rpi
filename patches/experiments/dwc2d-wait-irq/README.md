# No busy-waiting during USB transfers (experiment `dwc2d-wait-irq`)

Issue: [#104](https://github.com/redox-rpi/redox-rpi/issues/104). `dwc2d`
already uses the DWC2's buffer-DMA mode: the controller moves the data. But
`Dwc2::transfer_on` started a transfer and then spun until the channel
halted. Network receive waits for the interrupt (base `0022`), but every sent
frame, every 64 KiB USB stick chunk and every control request spun, so a
stick write kept a core busy.

- **base `0053`:** `transfer_on` spins for 50 µs, about a full-size
  network frame or a control request, and then waits for the channel's halt
  interrupt. dwc2d opens a second handle on its irq for this, with its own
  event queue and a 2 ms timer as the fallback (opened with `O_STAT`: the
  main loop's handle holds the reservation). While a transfer waits, only
  its channel raises the interrupt; the receive channel comes back once the
  channel is cleared and the interrupt acknowledged. The other channels are
  masked in their HCINTMSK: QEMU's model does not raise the interrupt again
  when HAINTMSK opens, so a frame received meanwhile waited (DHCP missed
  its first offer). Without an interrupt,
  transfers spin as before.
- A first try slept with naps from 50 µs to 1 ms. In QEMU dwc2d then
  sometimes never woke up from such a nap (state `US`, 4 of 4 boots), so
  DHCP and devshd never came up. Hence the interrupt, and no short sleeps.
- Applies on `usb-storage-more` (hotplug base `0031`–`0033`, experiments
  `usb-storage`, `usb-storage-more`): `--experiment usb-storage
  --experiment usb-storage-more --experiment dwc2d-wait-irq`.
- `scripts/measure-usb-cpu.sh [image]` (QEMU): dwc2d's CPU time (ACTTIME in
  `/scheme/sys/context`) and the speed for 30 s idle, a 64 MiB stick read, a
  32 MiB stick write and a 16 MiB TCP send to the host.

## Tests

- QEMU, 2026-10-31, `v0.3.0-dev-449-g461061e+x885b` against the baseline
  `v0.3.0-dev-441-g81d8d88+xf757` (#71's image), `scripts/measure-usb-cpu.sh`:

  | step | baseline | wait-irq |
  |---|---|---|
  | idle 30 s, dwc2d CPU | 0.40 s | 0.33 s |
  | stick read 64 MiB | 182 MB/s, 0.42 s | 128 MB/s, 0.49 s |
  | stick write 32 MiB | 160 MB/s, 0.30 s | 117 MB/s, 0.34 s |
  | TCP send 16 MiB | 0.20 MB/s, 57 s | 0.15 MB/s, 53 s |

  QEMU finishes a stick chunk almost at once, so waiting costs more than
  spinning there; on the Pi a 64 KiB chunk takes milliseconds of bus time.
  TCP send in QEMU is slow either way (about 7 ms per frame). Boot and
  devshd 3 of 3, `test-usb-storage.sh` 18/18, `test-hotplug.sh` 15/15,
  `test-keyboard.sh` 1/1.
- Pi: test #135, like for like against #71's image.
