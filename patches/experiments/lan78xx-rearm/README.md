# LAN7800 receive: next transfer first (experiment `lan78xx-rearm`)

Issue [#661](https://github.com/redox-rpi/redox-rpi/issues/661). A TCP
stream to the Pi 3B+ at about 11.5 MB/s has a steady ~1.4 % retransmits
(Pi test 591), on every image since at least v0.7.0-dev-9.

The LAN7800 receive FIFO is 12 KiB, which fills in about 1 ms at
100 Mbit/s. dwc2d keeps one bulk IN transfer in flight and starts the
next one only after the frames of the last one are copied out, after an
interrupt round trip to user space. When that takes longer than the FIFO
lasts, the chip drops frames.

- **base `0118`** (dwc2d): two receive buffers; when a transfer ends,
  the next one starts in the other buffer before the frames are taken
  out.

Goes together with `lan78xx-flow`. Test: the TCP stream of
`scripts/test-tcp-arp.sh` on the 3B+, retransmits per second against
main.
