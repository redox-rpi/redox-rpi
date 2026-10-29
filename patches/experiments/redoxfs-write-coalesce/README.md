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
transaction. Everything else (a read, `fstat`, `fsync`, any request on
another handle, a non-consecutive write, closing the handle) writes the
collected data first, so no other request sees the file without it.
A failed write of collected data is logged and returned by the next
`write` or `fsync` of that handle.

The on-disk format does not change.

Not yet: a timer. Data a program wrote and then left open without any
further request to the filesystem stays in memory (at most 128 KiB per
filesystem) until the next request of any program to that filesystem.
