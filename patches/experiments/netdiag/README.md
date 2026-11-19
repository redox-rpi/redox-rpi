# Network diagnosis: driver counters and `netdiag` (experiment `netdiag`)

Issue: [#566](https://github.com/redox-rpi/redox-rpi/issues/566). Needs
`dwc2d-netstats`. One snapshot answers "driver or netstack?" for a network
problem (#92, #119, #494, #540), instead of numbers collected by hand.

- **base `0100`** (driver-network): a network driver can give counters
  through the optional `NetworkAdapter::stats()`, read from
  `/scheme/network.<x>/stats`, one `name value` per line, taken at open.
  Drivers without it have no `stats` file (ENOENT).
- **base `0101`** (dwc2d): the counters of `dwc2d-netstats` in
  `/scheme/network.usb/stats` (same names as `/tmp/dwc2d-netstats`), plus
  one line per adapter, `adapter_<address> <kind> link <0|1> [active]`
  (the link is kept up to date only for the RTL815x).
- **`tools/netdiag`** (in `config.toml`):

  ```text
  netdiag                print the snapshot
  netdiag save <file>    store it
  netdiag diff <file>    what changed since the stored one
  ```

  A snapshot has every driver's stats (`network.<x>/<name>`), the
  interfaces' MAC and addresses and the routes from netcfg, the interrupt
  counts per line (`irq/<line>`, from `/scheme/sys/irq`) and the CPU time
  per process (`cpu/<name>`, ACTTIME from `/scheme/sys/context`, summed
  over its threads). `diff` prints numbers with their difference, CPU
  time in seconds, changed texts, and keys that came or went.

How to read a diff: `rx_frames` of the driver grows but the transfer does
not → netstack or above; `rx_errors` grows or `rx_frames` stands still
while the adapter's interrupt line grows → the driver or the adapter; a
process with CPU time close to the elapsed seconds is the limit.

bcmgenetd (Pi 400) gets its `stats()` in experiment `genet-stats`, on top
of `pi400`.

## Tests

- Host: `cargo test` in `tools/netdiag` (parsing and diff).
