# Experiment: dwc2d-async-link, ready without waiting for the link (#536)

Issue: [#536](https://github.com/redox-rpi/redox-rpi/issues/536), from the
boot-time analysis in rpi-open-firmware #44 (pi-tests Test 470).

## Problem

Since base `0034` (was experiment `dwc2d-boottime`), `dwc2d` waits at boot
for the first network link (up to 10 s, `BOOT_LINK_WAIT`) before it reports
ready, so DHCP does not start into a dead cable. `dwc2d` is a `notify`
service and init starts nothing else meanwhile. On the Pi 3B+ that wait is
the LAN7800's autonegotiation, about 3.1 s from the chip init to the link
(Test 470: `LAN78xx chip` at +3.38 s, `link up` at +6.49 s after kernel
start), also on images without a network stack.

## Change

Base patch [`0100`](base/0100-dwc2d-report-ready-without-waiting-for-the-network-l.patch):

- `dwc2d` reports ready as soon as a network adapter is up, or, without one,
  after the 1 s wait for late devices (`BOOT_CONNECT_WAIT`, unchanged). It
  prints `devices ready after N ms, link not up yet` when the link is not up.
- Until the adapter in use has its link, the frames the network stack sends
  wait in a queue in `dwc2d` (`UsbNet::held`, at most 32, the oldest go
  first) instead of going into the dead link. The link is read every 100 ms
  (`LINK_WATCH`); when it comes up, the queue is sent and `dwc2d: N of M
  held frame(s) sent` is printed. So dhcpd's first Discover leaves at
  link-up, within its 5 s reply timeout: still no DHCP start without a link
  (#217, #92), with no change in dhcpd or netstack.
- After 10 s without a link (`BOOT_LINK_WAIT`) the queue is dropped and
  `dwc2d: no link yet` is printed, as before; dhcpd retries as it does
  today.
- The MAC address the network stack keeps is still that of the adapter in
  use at ready time. If an RTL815x gets its link later, it takes over
  through the existing `select()` path with that address (as for an adapter
  plugged in after boot).

## Test only: dwc2d-async-link-test

QEMU's CDC Ethernet card has its link at once, so the queue is never used
there. `--experiment dwc2d-async-link-test` (patch `0101`, test-only, needs
this experiment) makes the card report no link until 2 s after the first frame was held.

```bash
scripts/build-image.sh --experiment timesync --experiment rtl8152 \
    --experiment dwc2d-async-link [--experiment dwc2d-async-link-test]
```

## Results

See below; Pi rows in pi-tests.
