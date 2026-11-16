# Boards (`BOARD`)

> **Moved to Feature issues (2026-11-16).** Each requirement below is now a
> Feature issue in redox-rpi, and the issue is the record ([README](README.md#feature-issues)).
> This file stays until the links to it are moved; do not edit it.

Topics: [boards](../topics/boards.md), [zero2w](../topics/zero2w.md).
Trace: [trace-v0.4.0.md](trace-v0.4.0.md).

The Pi 3 Model B+ is the reference board: every other requirement applies
to it unless it says otherwise.

### REQ-BOARD-1 Pi Zero 2 W with the same image

Feature issue: [#474](https://github.com/redox-rpi/redox-rpi/issues/474)

The release SD card image shall also boot on a Raspberry Pi Zero 2 W, with
USB keyboard, network through a USB Ethernet adapter, and NTP.

| Field | Value |
|---|---|
| Rationale | One image for both boards; the Zero 2 W is the small, cheap target. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 1.2 |
| Status | proposed |
| Since | v0.2.0 |
| Implemented by | #8 (closed) |
| Verification | test, person: checklist v0.4.0 tests 9 and 19, row 286 (rc2 visit, pass); v0.3.0: visit 193 |

Acceptance criteria:

1. The same card as in [REQ-BOOT-1](boot.md#req-boot-1-boot-from-the-sd-card)
   boots a Zero 2 W to `redox login:` on HDMI.
2. A USB keyboard types; an RTL8153 hub gets an address by DHCP and the
   clock comes from NTP
   ([REQ-NET-2](network.md#req-net-2-usb-ethernet-adapters),
   [REQ-TIME-1](time.md#req-time-1-clock-from-ntp)).
