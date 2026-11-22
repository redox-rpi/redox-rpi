# Network diagnosis: driver counters and `netdiag` (experiment `netdiag`)

Issue: [#566](https://github.com/redox-rpi/redox-rpi/issues/566). Needs
`dwc2d-netstats` and `netstack-watch` (a default test experiment). One snapshot answers "driver or netstack?" for a network
problem (#92, #119, #494, #540), instead of numbers collected by hand.

- **base `0100`** (driver-network): a network driver can give counters
  through the optional `NetworkAdapter::stats()`, read from
  `/scheme/network.<x>/stats`, one `name value` per line, taken at open.
  Drivers without it have no `stats` file (ENOENT).
- **base `0101`** (dwc2d): the counters of `dwc2d-netstats` in
  `/scheme/network.usb_<adapter>/stats` (same names as `/tmp/dwc2d-netstats`), plus
  one line per adapter, `adapter_<address> <kind> link <0|1> [active]`
  (the link is kept up to date only for the RTL815x).
- **base `0102`** (netstack, on top of `netstack-watch`):
  `/scheme/netcfg/stats` with netstack's counters: `frames_in` and where
  they went (`frames_in_ipv4`, `_arp`, `_not_ours`, `_malformed`,
  `_other`), `read_errors`, `frames_out`, `write_errors`, ARP
  (`arp_requests`, `arp_expired`, `arp_queue_full`, `arp_unresolved`: the
  last two are dropped packets), `router_rx_full` and `router_tx_full`
  (times a queue between the drivers and smoltcp was full), `no_route`,
  `sockets_tcp`, `sockets_udp`, `sockets_icmp` and `tcp_state_<state>`
  (a socket that is never freed shows as a growing count; REQ-NET-4
  criterion 2, `scripts/tcp-churn-check.py`), plus one line per connection,
  `tcp_<local port>_<remote>_<port> <state> rx_queue n/cap tx_queue n/cap`
  (refreshed by the main loop at most twice a second).
- **`tools/netdiag`** (in `config.toml`):

  ```text
  netdiag                print the snapshot
  netdiag save <file>    store it
  netdiag diff <file>    what changed since the stored one
  ```

  A snapshot has every driver's stats (`network.<x>/<name>`), netstack's
  counters (`netstack/<name>`), the
  interfaces' MAC and addresses and the routes from netcfg, the interrupt
  counts per line (`irq/<line>`, from `/scheme/sys/irq`) and the CPU time
  per process (`cpu/<name>`, ACTTIME from `/scheme/sys/context`, summed
  over its threads). `diff` prints numbers with their difference, CPU
  time in seconds, changed texts, and keys that came or went.

How to read a diff: the driver's `rx_frames` grows but netstack's
`frames_in` does not → netstack does not read (stuck, or `router_rx_full`);
`frames_in` grows but `frames_in_ipv4` not → frames for another address or
protocol; `arp_unresolved`, `arp_queue_full` or `write_errors` grow →
packets lost on the way out (#140); a TCP line's `tx_queue` full and not
moving → the peer does not take data (#119); `rx_errors` grows or `rx_frames` stands still
while the adapter's interrupt line grows → the driver or the adapter; a
process with CPU time close to the elapsed seconds is the limit.

bcmgenetd (Pi 400) gets its `stats()` in experiment `genet-stats`, on top
of `pi400`.

Build: `--experiment dwc2d-netstats --experiment netstack-watch
--experiment netdiag`, in this order: build-image.sh does not follow
`needs`, and adds the test experiment `netstack-watch` only after the
chosen ones otherwise.

A plain open of `/scheme/network.<x>/stats` (as `cat` does) fails with
ENODEV on these schemes; `netdiag` opens the scheme root as a directory and
the file in it, as netstack does with `mac`.

## Tests

- Host: `cargo test` in `tools/netdiag` (parsing and diff).
- QEMU raspi3b (`v0.6.0-dev-111-g394ca4a9+x1339`, CDC Ethernet): `netdiag
  save`, a devsh command, `netdiag diff` after 4.9 s showed
  `network.usb_ecm/rx_frames +48`, `tx_frames +46`, `irqs +48`, the
  matching interrupt line `irq/50 +48`, and CPU time per process.
- QEMU raspi3b (`v0.6.0-dev-115-g08842934+x1339`) with base `0102`:
  `cat /scheme/netcfg/stats` works (netcfg takes a plain open), and a
  `diff` over 1.7 s showed the driver's `rx_frames +49` matching
  netstack's `frames_in +49` (`frames_in_ipv4 +49`), `frames_out +47`
  against the driver's `tx_frames +47`, and the devsh connection as
  `tcp_2323_<host>_<port> ESTABLISHED rx_queue 0/65535 tx_queue 0/65535`.
- Pi 3B+ (pi-tests Test 512, `v0.6.0-dev-112-gc4cf59b2+x1339`, LAN7800):
  the snapshot has `network.usb_lan78xx/adapter_3 lan7800 link 1 active`
  and all counters. A 100 MiB download into `/scheme/null` (9.84 s,
  10.7 MB/s) between `save` and `diff`: `rx_frames +73608`, `tx_frames
  +11367`, `irqs` and `irq/50 +12462`, CPU time dwc2d +3.21 s and netstack
  +5.44 s, no errors. No serial capture in that run (the logger had no
  input), so no serial panic check.

CPU time per name is a sum over processes with that name: when one exits
between `save` and `diff` (a `login`), its line can go down a little.

