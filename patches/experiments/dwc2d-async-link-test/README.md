# Experiment: dwc2d-async-link-test (test only)

Needs [`dwc2d-async-link`](../dwc2d-async-link/README.md). QEMU's CDC
Ethernet card has its link at once; patch `0101` makes it report no link
at boot and a link 2 s after the first frame was held for it, so the frames
held for the link and their release can be seen in QEMU. Never for a Pi or a release.
