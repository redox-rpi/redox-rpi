# Shorter dwc2d boot (was experiment `dwc2d-boottime`)

Issue: #47 (boot time). **Default since the merge of this branch:** the
patch is base `0034` in `patches/base/`, after the hotplug patches
`0031`–`0033`, which it needs: devices that connect late are found by the
hotplug rescan. `--experiment dwc2d-boottime` is still accepted and adds
nothing. Below, `0037` is the number it had as an experiment.

On the Pi 3B+, `40_dwc2d.service` took 6.2 s until ready (test 53),
which is 60 % of the time from kernel start to `redox login:`:

| From the boot log (test 53) | Time |
|---|---|
| controller, root port reset | 0.25 s |
| hub 1 (4 ports): wait for empty ports to connect | 1.2 s |
| hub 2 (3 ports): the same | 1.15 s |
| hub 4 (4 ports, the Anker hub): the same | 1.2 s |
| LAN7800 bring-up, then waiting for its link | 0.15 s + 1.71 s |

- **base `0037`:** a hub takes the ports that are connected after
  power-good and moves on, instead of waiting a second for each empty
  port to connect. The network adapters start without waiting for their
  link. dwc2d then waits for the first link (up to 10 s, as before) and
  runs the hotplug rescan meanwhile, so devices that connect late and the
  LAN7800's autonegotiation overlap. Without a network adapter it looks
  for late devices for 1 s. The adapter whose MAC address the network
  stack keeps is picked after that wait, by link, as before. The LAN7800's
  link is now followed after boot too, like the RTL815x's.
  - dwc2d prints `devices ready after N ms` when it reports ready.
  - Expected on the Pi: about 2.2 s instead of 6.2 s (bounded by the
    LAN7800's autonegotiation).

## Tests

- **QEMU** (`v0.3.0-dev-176-g752ac70+x8057`): `devices ready after`
  461, 477 and 521 ms (before: 1.6 s until ready); `scripts/test-hotplug.sh`
  15 of 15, three times.
- **Pi** (test #67, `v0.3.0-dev-176-g752ac70+x8057`): **pass**.
  `devices ready after 3294 ms` in both boots instead of 6.2 s; about
  2.1 s of it is the LAN7800's autonegotiation. Kernel to login 7.4 s
  instead of 10.3 s. Standard checks, `scripts/pi-hotplug-test.sh` 15/15
  twice, keyboard cycle.

## Merge to main

Pull request on top of `usb-hotplug`, with main merged in (2026-10-30).
The patch applies unchanged (same patch ID).

- **Proven on the Pi:** test #67 above.
- **QEMU after merging main** (`v0.3.0-dev-325-g9c7ecdb+x8057`):
  `scripts/test-hotplug.sh` 15/15. Ready after 455 ms with a network card
  and 1504 ms without one (the 1 s wait for late devices).
- **Still needs a person:** nothing.
