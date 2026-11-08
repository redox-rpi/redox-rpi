# Firmware interface and CPU clock (`FW`)

Topic: [firmware](../topics/firmware.md). Trace: [trace.md](trace.md).

### REQ-FW-1 Full CPU speed without throttling

The system shall run the ARM cores up to their maximum clock (1400 MHz on
the Pi 3B+) and log undervoltage or overheating.

| Field | Value |
|---|---|
| Rationale | At the firmware's 600 MHz the Pi is half as fast. |
| Priority | must |
| Kind | quality |
| Level | release |
| PBS | 2.3.8 |
| Status | proposed |
| Since | v0.3.0 |
| Implemented by | #20, base `0021` |
| Verification | test, automated: checklist v0.4.0 test 2, row 285 |

Acceptance criteria:

1. Under load (the 1000 starts of test 1),
   `/scheme/rpi-firmware` reports an ARM clock of 1400 MHz.
2. With a good power supply, no throttling or undervoltage line in the log.

### REQ-FW-2 CPU clock follows the load

By default the CPU clock shall follow the load; `/etc/rpi-firmware/governor`
shall set another mode at boot.

| Field | Value |
|---|---|
| Rationale | Lower clock at idle saves power and heat without slowing work. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.3.8, 2.6.1 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #23 (open, milestone v0.4.0), PR #147, base `0040`, kernel `0040` |
| Verification | test, automated: checklist v0.4.0 test 13, row 285 (criterion 1); row 161 (criterion 2, dev build) |

Acceptance criteria:

1. Without the file, `/scheme/rpi-firmware/governor` shows `ondemand`; at
   idle the clock is 600 MHz most of the time, during the starts of test 1
   1400 MHz.
2. With `performance`, `powersave` or a rate in the file, the clock is fixed
   at that rate after boot.

### REQ-FW-3 Reboot

`reboot` as root shall restart the Pi.

| Field | Value |
|---|---|
| Rationale | The Pi has no power button; updates and tests need a restart. |
| Priority | must |
| Kind | functional |
| Level | release |
| PBS | 2.2.1, 2.5.2 |
| Status | proposed |
| Since | v0.2.0 |
| Implemented by | kernel `0002` (watchdog), #12; #7 (power off, open) |
| Verification | test, automated: checklist v0.4.0 test 5, row 285 |

Acceptance criteria:

1. `reboot` 5 times in a row: the Pi is back at `redox login:` each time.

### REQ-FW-4 Firmware information

The system shall show the board model, memory, clocks, temperature and
throttling state from the VideoCore firmware under `/scheme/rpi-firmware`.

| Field | Value |
|---|---|
| Rationale | Users and tests need to know the board and its health. |
| Priority | should |
| Kind | functional |
| Level | release |
| PBS | 2.3.8 |
| Status | proposed |
| Since | v0.2.0 |
| Implemented by | base `0018`, [rpi-firmware.md](../rpi-firmware.md) |
| Verification | test, automated: a new `pi-test.py` check, so the nightly runs it too (agreed with V&V 2026-11-08, not written yet); the clock is already read by test 2 in row 285 |

Acceptance criteria:

1. `/scheme/rpi-firmware/board` has a model line naming the Pi 3 Model B+
   and a memory line (exact text confirmed by V&V on the Pi).
2. `/scheme/rpi-firmware/temperature` is between 20 000 and 85 000
   (millidegrees) at idle.
3. `/scheme/rpi-firmware/throttled` reads as hex. Bits 0–3 (current
   undervoltage and throttling) are 0 at idle; a set bit is a warning, not a
   fail, because the cause is the power supply.
