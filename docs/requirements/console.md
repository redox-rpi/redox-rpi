# Console (`CON`)

Topics: [boot](../topics/boot.md) (boot and console),
[stability](../topics/stability.md) (serial input). Trace: [trace.md](trace.md).

### REQ-CON-1 HDMI console with login

The system shall show a text console on HDMI where a user logs in with a
USB keyboard.

| Field | Value |
|---|---|
| Rationale | A Pi with a screen and a keyboard is the normal way to use the SD card image. |
| Priority | must |
| Level | release |
| Status | proposed |
| Since | v0.1.0-alpha (screen), v0.2.0 (keyboard) |
| Issues | — (framebuffer inherited from UEFI; keyboard: [REQ-USB-1](usb.md#req-usb-1-usb-keyboard)) |
| Verified by | checklist v0.4.0 test 7, row 286 (queued); v0.3.0: visit 193 |

Acceptance criteria:

1. `redox login:` appears on the HDMI screen.
2. Login works as `root` with password `password` and as `user` without a
   password (the defaults in the release notes); a shell prompt follows.
3. `Super`+`F1` and `Super`+`F2` switch between the virtual terminals and
   back, and typing works on each.

### REQ-CON-2 Serial console

The system shall offer a login on the PL011 serial port (GPIO 14/15,
115200 baud) that takes fast input without losing characters.

| Field | Value |
|---|---|
| Rationale | The serial console is the debug and test access when the network is down. |
| Priority | must (login), should (fast input) |
| Level | release |
| Status | proposed |
| Since | v0.1.0-alpha (login), v0.3.0 (fast input) |
| Issues | #28 (open), #32, base `0024`, kernel `0007` |
| Verified by | checklist v0.4.0 test 11, row 285: **fails** (19 of 20, #28 known issue) |

Acceptance criteria:

1. Boot messages and `redox login:` appear on the serial port; login and a
   shell work there.
2. `scripts/serial-input-test.sh 20`: 20 of 20 lines of 200 characters sent
   at full speed arrive unchanged.
