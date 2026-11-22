# Trace: v0.6.0

Status: **started**, 2026-11-22. Owner: the Requirements agent.

Requirement → issue → test → state for the release **v0.6.0**. States and
the meaning of "row" are in the [README](README.md#trace). So far only the
network requirements are filled in; the other areas follow before the
release step "Untested `must` criteria" of the
[v0.6.0 checklist](../releases/v0.6.0-checklist.md).

Evidence so far:

- [Test 571](https://github.com/redox-rpi/pi-tests/blob/main/reports/571.md):
  v0.6.0-rc2 automated, Pi 3B+, netboot image (build
  `v0.6.0-dev-129-g988c8354+xcbcb`).
- [Test 574](https://github.com/redox-rpi/pi-tests/blob/main/reports/574.md):
  v0.6.0-rc3 person test, Zero 2 W, SD card (build
  `v0.6.0-dev-135-g0897391f+x6a9f`). Part 2: LAN over a USB Realtek
  adapter on a hub HAT, and NTP.

## Network

REQ-NET-1 and REQ-NET-2 were reworded on 2026-11-22 (maintainer's
decision): DHCP counts from link-up and names no adapter; the supported
adapters are REQ-NET-2.

| Requirement | Prio | Level | Issues | Test | State |
|---|---|---|---|---|---|
| [REQ-NET-1](https://github.com/redox-rpi/redox-rpi/issues/491) DHCP address on every supported adapter | must | release | #217, #92 | Test 571 (3B+, rc2), Test 574 (Zero 2 W, rc3) | 1 verified on both boards (Zero: about 10 s); 4 verified (3B+); 2, 3 not tested; 5, 6 **gap** (#259) |
| [REQ-NET-2](https://github.com/redox-rpi/redox-rpi/issues/492) Supported Ethernet adapters | may (Low) | release | base `0020` | Test 571 (LAN7800), Test 574 (USB Realtek) | verified (1, 2); proposed wording |
