# USB (`USB`)

Topic: [usb](../topics/usb.md). Trace: [trace.md](trace.md).

### REQ-USB-1 USB keyboard

The system shall take input from a USB boot-protocol keyboard on any USB
port of the Pi 3B+ (behind its onboard hubs) and on the Zero 2 W.

| Field | Value |
|---|---|
| Rationale | Without a keyboard there is no local use of the Pi. |
| Priority | must |
| Kind | functional |
| Level | release |
| PBS | 2.3.2 |
| Status | proposed |
| Since | v0.2.0 |
| Implemented by | `dwc2d` (base patches), #29 (interrupts) |
| Verification | test, person: checklist v0.4.0 tests 7 and 9, row 286 (queued); QEMU `scripts/test-keyboard.sh` |

Acceptance criteria:

1. A low- or full-speed keyboard plugged in before power-on types on the
   HDMI console (US layout), on every USB port.
2. A held key repeats.

### REQ-USB-2 USB hotplug

The system shall use a keyboard or network adapter plugged in after boot,
and keep working when one is unplugged and plugged in again.

| Field | Value |
|---|---|
| Rationale | Users plug and unplug devices; a reboot for each is not acceptable. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.3.2 |
| Status | proposed |
| Since | v0.3.0 |
| Implemented by | #17 (closed), base `0031`–`0033` |
| Verification | test, person: checklist v0.4.0 test 8, row 286 (queued); rows 35 and 67 (`scripts/pi-hotplug-test.sh`) |

Acceptance criteria:

1. Keyboard out and in again in the same boot: typing works within 5 s
   (proposed; dwc2d rescans every second).
2. Ethernet cable plugged in after boot: the Pi has an address within 60 s.
3. No `dwc2d` panic in the serial log.

### REQ-USB-3 USB mouse

The system shall report a USB mouse or tablet as input events.

| Field | Value |
|---|---|
| Rationale | The optional desktop ([REQ-GFX-1](graphics.md#req-gfx-1-optional-desktop-from-the-console-beta)) needs a pointer. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.3.2 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #48 (still open, milestone v0.5.0, although PR #223 is in rc1), PR #223, base `0059` |
| Verification | test, person: checklist v0.4.0 test 21, row 286 (queued); QEMU `scripts/test-usb-mouse.sh` |

Acceptance criteria:

1. A mouse plugged in before or after boot is logged by `dwc2d` as an input
   device.
2. Moving it and clicking reaches `inputd` (in the desktop: the pointer
   moves and clicks).
3. No `dwc2d` panic.

### REQ-USB-4 Low idle load of the USB driver

The USB host driver shall not keep a CPU core busy when nothing happens on
the bus.

| Field | Value |
|---|---|
| Rationale | A busy core costs power and heat and slows everything else. |
| Priority | should |
| Kind | quality |
| Level | release |
| PBS | 2.3.2 |
| Status | proposed |
| Since | v0.3.0 |
| Implemented by | #29 (closed), #214 (open), #104 (open) |
| Verification | test, automated: row 5 (dev build, v0.3.0); **gap** in the v0.4.0 checklist |

Acceptance criteria:

1. At idle after boot, with keyboard and Ethernet, `dwc2d` uses at most 10 %
   of one core (proposed; about 5 % in row 5), measured over 60 s.
