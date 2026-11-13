# netstack and its kernel side: pointers for upstream

Notes for the user, who reports to Redox. Checked against upstream on
2026-10-31: base `main` at `2ec1b19d` and kernel `master` at `1a1295b0`.
Both problems are still there.

## `ls` of a netcfg directory fails with `Not a directory`

- **What breaks:** `ls /scheme/netcfg` and `ls /scheme/netcfg/ifaces/eth0`
  fail with `Not a directory (os error 20)`. Reading a single node such as
  `cat /scheme/netcfg/ifaces/eth0/mac` works. Found in our Pi test 122
  (our [#105](https://github.com/redox-rpi/redox-rpi/issues/105)).
- **Why:** the netcfg scheme says a directory is a directory but can't list
  it. In `netstack/src/scheme/netcfg/mod.rs`:
  - `fstat` (line 535) sets `MODE_DIR` for a directory node (line 541).
  - `impl SchemeSync for NetCfgSchemeInner` (line 399) has no `getdents`,
    so the default answers `ENOTDIR`, and `ls` gives up.
  - A directory node already has its child names: opening it (around
    line 432) reads them into the handle's buffer, one per line, and that
    is what `cat /scheme/netcfg` prints.
- **Reproduce:** any Redox with netstack (QEMU is enough):
  `ls /scheme/netcfg` gives the error; `cat /scheme/netcfg` lists the names.
- **Our fix:** base patch `0056`
  ([PR #107](https://github.com/redox-rpi/redox-rpi/pull/107)). It adds a
  `getdents` that returns those same names, one `DirEntry` per line, with
  the line number as the offset; it returns `ENOTDIR` for a node that isn't
  a directory. About 20 lines in that one file.
- **Tested:** QEMU, Pi 3B+ test 128 (`ls` of `/scheme/netcfg`, `ifaces`,
  `ifaces/eth0`, `ifaces/eth0/addr` and `route` lists the names).

## A TCP read before the handshake ends returns end of file

Our [#404](https://github.com/redox-rpi/redox-rpi/issues/404). Checked against the base we pin, `eddcaa21`.

- **What breaks:** `nc <host>:<port>` on the Pi often got none of the
  server's reply when the server answered and closed at once. Only nc's own
  `Remote host:` line reached its output. To reproduce in QEMU:
  `nc 192.0.2.1:80` (an address that never answers) exits at once with
  status 0 instead of waiting for the connection.
- **Why:** relibc's `connect()` dups the socket with the address, and
  netstack's TCP `open` calls smoltcp's `connect` and returns. The socket is
  still in SYN-SENT. smoltcp's `may_recv()` is false there, so
  `read_buf` in `netstack/src/scheme/tcp.rs` returns 0 when the read comes
  before the SYN-ACK. nc's read thread takes that as end of file and exits.
  The event path (`can_recv || !may_recv` in `socket.rs`) uses the
  scheme's `may_recv` and gets the same answer.
- **Our fix:** default base patch `0097`: the scheme's `may_recv` for TCP
  is also true in SYN-SENT and SYN-RECEIVED, and `read_buf` calls it by its
  trait name. A plain `self.may_recv()` there calls smoltcp's method of the
  same name, which wins over the trait method. The read now waits for
  data. A refused connection ends in Closed, and the read then fails with
  ENOTCONN. A blocking `connect()` that waits for the handshake, as POSIX
  has it, would be the fuller fix.

## Every open of a scheme's root leaves a page mapped in the daemon (kernel)

- **What breaks:** a scheme daemon's memory grows 4 KiB on every open of
  the scheme's root, which is an open with an empty path, and never gives it
  back.
  - netstack grows per TCP connection, because relibc's `socket()` opens
    `/scheme/tcp`.
  - randd grows per random read, so twice per TLS handshake.
  - It shows as the `SHARED` column of `/scheme/sys/context`. procmgr and
    zerod grew the same way. Found while chasing a netstack memory growth
    (our [#88](https://github.com/redox-rpi/redox-rpi/issues/88), Pi
    test 111).
- **Why:** in the kernel's `src/scheme/user.rs`, `copy_and_capture_tail`
  (line 405) copies the path into one page and maps that page into the
  daemon (`mmap_anywhere` with `ONE` page). It then computes the span to
  unmap later from the path's length:
  `page_range_containing(base, len)` (lines 460–463). For an empty path,
  `len` is 0, so the span is empty, and the page is never unmapped.
- **Reproduce:** QEMU, then
  `grep netstack /scheme/sys/context; for i in 1..201; ls -d /scheme/netcfg > /dev/null; end; grep netstack /scheme/sys/context`:
  `SHARED` grows about 800 KB. The same with `head -c 16 /scheme/rand` and
  randd. Deeper paths (`/scheme/netcfg/ifaces/eth0/mac`) don't leak.
- **Our fix:** kernel patch `0057`
  ([PR #114](https://github.com/redox-rpi/redox-rpi/pull/114)), one line:
  the span is always `PageSpan::new(dst_page, 1)`, the one page that was
  mapped.
- **Tested:** QEMU (200 runs each of `ls -d`, `head` and MQTT over TLS and
  plain: no growth, where each grew about 800 KB before); Pi 3B+ test 136
  (netstack `SHARED` 16.0 KB and randd 0 B, before and after).
