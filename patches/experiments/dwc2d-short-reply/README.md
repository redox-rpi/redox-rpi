# Short USB replies (experiment `dwc2d-short-reply`)

Pi test 98 (main-based image `v0.3.0-dev-362-g4ddab1f+x0937`) found a
`dwc2d` panic in 1 of 2 boots, and then there was no network adapter:

    panicked at drivers/usb/dwc2d/src/enumerate.rs:173:40:
    index out of bounds: the len is 0 but the index is 7

Line 173 reads the max packet size from the first 8 bytes of the device
descriptor, right after a port reset. The reply's data stage was empty,
and the code indexed it without checking its length. main and v0.3.0-rc2
have the same code; `usb-hotplug` has it at line 216.

- **base `0037`:** the device descriptor (8 and 18 bytes), the hub port
  status (4) and the hub descriptor (at least 6) are fetched with
  `Bus::request_min`. A reply shorter than the fields read from it is
  asked for again, up to three times 10 ms apart, with a log line
  `request [...] to address N: x of y bytes (attempt n)`. After that the
  request fails with `Error::Short` instead of panicking. A device behind
  a hub that still fails is logged and skipped; the others come up as
  before.
- It applies on main and on top of `usb-hotplug` (base `0031`–`0033`)
  without changes.

## Tests

- QEMU (`v0.3.0-dev-419-g65391ae+x1a87`), two boots: hub, keyboard and
  network card enumerated, network up, no panic, no retry line. QEMU's
  devices never send a short reply, so the retry itself does not run
  there.
- Pi: test #105 (Pi test agent), 5 boots, looking for the new
  log line.
