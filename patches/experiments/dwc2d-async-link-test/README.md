# Experiment: dwc2d-async-link-test (test only)

Needs [`dwc2d-async-link`](../dwc2d-async-link/README.md). QEMU's CDC
Ethernet card has its link at once; patch `0101` makes it report no link
until 3 s after `dwc2d` found it, like the LAN7800's autonegotiation on the
Pi 3B+, so the frames held for the link and their release can be seen in
QEMU. Never for a Pi or a release.
