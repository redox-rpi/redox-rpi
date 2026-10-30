# Network adapter counters (experiment `dwc2d-netstats`, diagnosis)

Issue: [#92](https://github.com/redox-rpi/redox-rpi/issues/92). The network
sometimes stops a few minutes after boot while the serial console keeps
working, and nothing is logged. This experiment tells the two sides apart:
do frames still come from the USB adapter (then netstack or the kernel stops
reading them), or do they stop in `dwc2d`?

- **base `0052`:** every 5 s `dwc2d` writes `/tmp/dwc2d-netstats`, one
  `name value` per line:
  - `uptime_ms`: since the counters started (shortly after boot)
  - `rx_frames`, `rx_errors`, `rx_last_error`, `rx_last_ms`: frames from the
    adapter, failed bulk IN transfers, the last error code, the time of the
    last frame
  - `tx_frames`, `tx_errors`, `tx_last_error`, `tx_last_ms`: the same for
    frames to the adapter
  - `irqs`: controller interrupts, `ticks`: requests from netstack
  - Error codes: HCINT bits of a halted channel (e.g. `0x80` transaction
    error, `0x100` babble, `0x400` data toggle error), `0x10000` stall,
    `0x20000` timeout, `0x30000` short reply.
- LAN7800 and CDC Ethernet are counted; RTL8152 is not. No log output (a
  println after boot can block `dwc2d`). The link state is not in it: reading
  it needs a control transfer, which this diagnosis leaves out.

How to read it when the network is gone: `rx_last_ms` far behind `uptime_ms`
with `rx_errors` growing → the adapter or `dwc2d`; `rx_frames` still growing
→ netstack or the kernel; `uptime_ms` frozen → `dwc2d` itself is stuck (or
`/tmp`). `ticks` frozen while `rx_frames` grows → netstack stopped asking.

Build: `--experiment dwc2d-netstats` on main.

## Tests

- QEMU: see below.
