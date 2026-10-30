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

## Tests

- `scripts/test-scheme-list.sh [image]` needs the test experiment
  `scheme-list-stress` (never in a release): a unit started by init, in
  init's namespace where netstack lives, starts 40 `ramfs` instances one
  by one and lists `/scheme` five times after each. The script checks over
  devsh that all 200 listings contain the network scheme exactly once.

Build for the check:
`scripts/build-image.sh --experiment scheme-list-stress` (without the fix)
and `--experiment ns-list-snapshot --experiment scheme-list-stress` (with
it).
