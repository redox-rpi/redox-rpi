# Pi 400 Ethernet counters for netdiag (experiment `genet-stats`)

Issue: [#566](https://github.com/redox-rpi/redox-rpi/issues/566). Needs
`pi400` (its `base/0002` adds `bcmgenetd`) and `netdiag` (the `stats`
file in driver-network).

- **base `0110`**: `bcmgenetd` fills `/scheme/network.genet/stats`:
  - `link` (speed or `down`), `rx_frames`, `rx_bad` (marked bad or split
    by the controller), `rx_too_big`, `rx_pending` (waiting in the ring),
    `tx_frames`, `tx_full` (tries refused by a full ring), `tx_dropped`
    (frames given up after 10 ms of a full ring), `tx_in_flight`, `irqs`,
    `link_changes`;
  - the controller's own counters (offsets as Linux `bcmgenet`):
    `hw_rx_pkts`, `hw_rx_good`, `hw_rx_fcs_errors`, `hw_rx_oversize`,
    `hw_tx_pkts` (MIB, cleared at reset), `hw_rbuf_overflows` and
    `hw_rbuf_errors` (receive buffer).

`hw_rx_pkts` against `rx_frames` tells frames lost between the wire and
the driver (`hw_rbuf_overflows` when the ring was not emptied in time)
from frames lost further up.

Build: `--experiment pi400 --experiment dwc2d-netstats --experiment
netstack-watch --experiment netdiag --experiment genet-stats`.

## Tests

- Host: `cargo test --lib` in `drivers/net/bcmgenetd` (counters and the
  hardware counter reads against a fake register file).
