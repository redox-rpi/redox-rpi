# Complete /scheme listings (experiment `ns-list-snapshot`)

Issue: see the pull request. Found while checking the USB hotplug pull
request #68: in about 1 of 11 QEMU boots without a network card, netstack
said "no network adapter found" although `dwc2d` had registered
`network.usb` before it reported ready.

- **base `0036`:** `initnsmgr` (the namespace manager in `bootstrap`) keeps
  a namespace's scheme names in a hash map, and `getdents` on `/scheme`
  paged through that map by position. relibc reads a directory 512 bytes
  at a time, so listing about 40 schemes takes several `getdents` calls.
  When a scheme is registered between two of them and the map grows, it is
  rebuilt in another order, and the listing skips or repeats names. At
  boot, `sudo --daemon` (`oneshot_async`) registers `sudo` while netstack,
  the next unit, lists `/scheme` for a `network.*` scheme. Each listing now
  goes through the names as they were when it started, sorted, and the
  copy is dropped when the directory is closed.

## Why it is rare

`model/` is a small host program (`cargo run` in that folder) that does
what `initnsmgr` did: the names in a hashbrown 0.15.5 map (the version
`bootstrap` uses), listed in 512-byte pages, with `sudo` registered after
the first, second or third page. With 15 to 61 schemes, the listing loses
names only when the map holds exactly 28 or 56 schemes: then the insert
grows the table and reorders it, and 2 to 12 names go missing, sometimes
`network.usb`. Which names depends on the hash seed. At boot this needs
the scheme count to hit such a threshold just when `sudo` registers in the
middle of netstack's listing, which matches the 1 in 36 QEMU boots seen.

Starting `ramfs` instances to make the race frequent did not work in
QEMU: started from a shell or a unit, `ramfs` panicked ("Interrupted
system call").

## Tests

- `scripts/test-scheme-list.sh [image]` (QEMU, image with this
  experiment): netstack found the network scheme, `/scheme` lists every
  scheme once, sorted, and twice the same.
- Pi: a normal boot with this experiment (standard checks); the race is
  too rare to show there directly.
