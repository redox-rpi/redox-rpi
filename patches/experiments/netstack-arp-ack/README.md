# Stale ARP entries and reads after a reset (experiment `netstack-arp-ack`)

Issues: [#140](https://github.com/redox-rpi/redox-rpi/issues/140),
[#141](https://github.com/redox-rpi/redox-rpi/issues/141); both seen in
#119.

**Base `0082`**, two small fixes in netstack:

- **ARP:** an entry expired 60 s after it was learned. Every packet to that
  host then waited in a queue of 10 until the ARP reply came, so a busy
  download dropped ACKs once a minute (`Dropped packet on eth0 because
  waiting queue was full`). Now an expired entry stays in use for up to 10 s
  more while an ARP request asks again, like Linux's "stale" state. The reply
  renews it. An entry that gets no answer in that time is removed, as before.
- **Read after a reset:** when the other side reset a TCP connection, a read
  returned `ENOTCONN` at once, although data that arrived before the reset
  was still in the receive buffer. `read_buf` now hands out that data first,
  and returns `ENOTCONN` only when the buffer is empty.

Build: `--experiment netstack-arp-ack` on main; add `--experiment netget`
for the test downloader.

## Tests
