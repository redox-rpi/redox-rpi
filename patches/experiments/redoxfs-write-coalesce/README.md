# redoxfs-write-coalesce

Prototype for [#16](https://github.com/redox-rpi/redox-rpi/issues/16):
RedoxFS writes about 28 times the data for a file written in 4 KiB calls.

## Cause

Every `write` call to the RedoxFS scheme is one transaction
(`FileSystem::tx`), and every transaction is written out at its end. A file
keeps its data in records of up to 128 KiB (`RECORD_LEVEL` 5), and a record
is copy-on-write: changing 4 KiB of it writes the record again, as large as
the data in it so far (rounded up to a power of two). Writing a file
from the start in 4 KiB calls, the 32 calls of one record write
4 + 8 + 16 + 16 + 4×32 + 8×64 + 16×128 KiB = 2732 KiB for 128 KiB, 21.3
times. Each transaction also writes the node, the four blocks of the tree
path to it, the allocator log block and the header, about 7 blocks of 4 KiB
per call, another 7 times. Together 28.3, the 28.4 measured in QEMU
(random data, so the lz4 compression of records saves nothing).

## Change

The scheme collects the consecutive writes of one handle: the first write
of a run goes to the disk at once (so permission and other errors come back
to the caller), the following ones that start where the last one ended stay
in memory until a record is complete (128 KiB), and are written then in one
transaction. Everything else (a read of that file, `fstat`, `fsync`, any
other request on another handle, a non-consecutive write, closing the
handle) writes the collected data first, so no other request sees the file
without it. Reads of other files do not (patch `0002`): a copy reads the
source between the writes to the copy, and on the Pi (test #46) a copy in
4 KiB blocks still took 12 s for 1 MiB while every read cut the run.
A failed write of collected data is logged and returned by the next
`write` or `fsync` of that handle.

The on-disk format does not change.

Collected data goes to the disk at the latest 2 s after it came in
(patch `0003`), also when the program writes nothing more and keeps the
file open: the scheme socket is non-blocking, and with no request waiting
redoxfs sleeps on an event queue with the socket and a monotonic timer. If
the timer cannot be opened it logs `no timer for collected writes` and
works as before, where such data waited for the next request.

## Results

QEMU raspi3b, 2026-10-29, `scripts/redoxfs-write-amp.py` (with the console
marker typed again when the serial console drops a character), both images
built from branch `t16-write-amp` 2ce732b, three runs each, the same numbers
in all three:

| Call size | Without (`v0.3.0-dev-164-g2ce732b`) | With (`v0.3.0-dev-164-g2ce732b+x8cc4`) |
|---|---|---|
| 4 KiB | 29814784 bytes, **28.4** | 1351680 bytes, **1.3** |
| 64 KiB | 2072576 bytes, 2.0 | 1413120 bytes, 1.3 |
| 1 MiB | 1114112 bytes, 1.1 | 1114112 bytes, 1.1 |

Correctness, in the guest over devshd: a 1 MiB random file copied with
`dd` in calls of 4 KiB, 1000 bytes and 100000 bytes, and 512 bytes with and
without `conv=fsync`, gives the same SHA-256 every time; 300 `echo >> log`
appends give 300 lines. The same files read from the card image on the host
(`tools/redoxfs-file`) match. A QEMU that was killed in the middle of a copy
in 1000-byte calls left an exact prefix of the source on the card.

Pi test #46 (2026-10-29, `v0.3.0-dev-164-g2ce732b+x8cc4`, patch `0001`
only): 1 MiB in 4 KiB calls with `fsync` 0.4 s, against 10.1 s on the
stable image and the same card; copies and appends intact, also after a
reset. A file-to-file copy in small blocks was still slow (1 MiB at
`bs=4k` 12 s), because every read of the source cut the run.

With patch `0002`, QEMU, 1 MiB copied from a file to a file on the same
filesystem with `conv=fsync` (`v0.3.0-dev-166-g403f6c1+x8cc4` against
`v0.3.0-dev-164-g2ce732b+x8cc4`):

| Block size | `0001` only | `0001` + `0002` |
|---|---|---|
| 4 KiB | 28.9 s | 0.94 s |
| 1000 bytes | 98.0 s | 1.6 s |

Both copies have the source's SHA-256.
