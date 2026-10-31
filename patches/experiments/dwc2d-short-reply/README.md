# Short USB replies (was experiment `dwc2d-short-reply`)

**Default since PR #84:** base `0037` (moved to `patches/base/`) is in
every image. `--experiment dwc2d-short-reply` is still accepted and does
nothing, so older commands keep working. This page stays as the patch's
documentation.

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
- PR #84 merged first; PR #68 (hotplug, base `0031`–`0033`) refreshed
  `0037` against those, since both change `enumerate.rs`. Same code, other
  context lines. PR #70 (USB serial, base `0035`) refreshed it on top
  of `0035`.

## Tests

- QEMU (`v0.3.0-dev-419-g65391ae+x1a87`), two boots: hub, keyboard and
  network card enumerated, network up, no panic, no retry line. QEMU's
  devices never send a short reply, so the retry itself does not run
  there.
- Pi: test 105 ([report](https://github.com/redox-rpi/redox-rpi/blob/main/docs/test-reports.md#2026-10-30-v030-dev-419-g65391aex1a87), Pi test agent): **5 of 5 boots without
  a panic**, network up and the standard checks passing every time. No
  boot printed the retry line: no short reply came, so this shows nothing
  breaks, not that the retry catches the case (test 98 hit it in 1 of 2
  boots).
