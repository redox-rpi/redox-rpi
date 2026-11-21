# Experiment: dwc2d-async-link-test (test only)

Needs [`dwc2d-async-link`](../dwc2d-async-link/README.md). QEMU's CDC
Ethernet cards have their link at once; patch `0900` makes them report no
link at boot, and a link 2 s after the first frame was held (one card) or
2 s after `dwc2d` began to wait for the first link (several cards), so both
paths can be seen in QEMU. Never for a Pi or a release.
