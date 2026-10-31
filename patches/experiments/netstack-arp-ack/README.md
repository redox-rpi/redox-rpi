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

`rstserver.py PORT BODY` (here, for the host) answers each connection with an
HTTP header and offers BODY bytes, then resets the connection after 2 s. With
a body larger than the receive window (64 KiB) and a slow reader
(`netget URL --slow 5000`), the reader still has data buffered when the reset
comes.

- QEMU `raspi3b` (`v0.3.0-dev-795-g187edb8+xff6c`, with `netget`), 382 MB
  download with a capture of the USB network card: ARP request and reply
  every 60 s (56, 116, 176, 236, 296 s) while the download continues; no
  `Dropped packet` line; 382156480 bytes complete. QEMU's user network
  answers ARP at once, so the old code shows the same capture; the
  difference only shows where the reply takes a while (the Pi: test 141 had
  two `waiting queue was full` drops).
- QEMU can't test the reset: its user network ends the guest's connection
  itself and turns the host's reset into a normal close. The old and the new
  image both read to the end of the stream. On the Pi the connection goes
  straight to homenodellm, so the reset arrives as one.
- Pi 3B+: test plan row.
