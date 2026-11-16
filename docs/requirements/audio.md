# Audio (`AUDIO`)

> **Moved to Feature issues (2026-11-16).** Each requirement below is now a
> Feature issue in redox-rpi, and the issue is the record ([README](README.md#feature-issues)).
> This file stays until the links to it are moved; do not edit it.

Topic: [audio](../topics/audio.md). Trace: [trace-v0.4.0.md](trace-v0.4.0.md).

### REQ-AUDIO-1 Sound from the headphone jack (beta)

Feature issue: [#250](https://github.com/redox-rpi/redox-rpi/issues/250)

The system shall play 16-bit stereo sound at 44.1 kHz written to
`/scheme/audio` through the Pi 3B+'s 3.5 mm jack, in real time.

| Field | Value |
|---|---|
| Rationale | Sound is a basic feature of a computer; the jack needs no extra hardware. |
| Priority | should |
| Kind | functional |
| Level | beta (about 11 bits, PWM keeps running after silence, faint hiss) |
| PBS | 2.3.10, 2.4.5 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #250 (open, milestone v0.4.0), PR #251, base `0086`, `0030`, kernel `0050` |
| Verification | test, automated: checklist v0.4.0 test 16, rows 292 (v0.4.0 build) and 285 (rc1) |

Acceptance criteria:

1. `cat /scheme/audiohw/stats` shows a rate within 2 % of 44 100 frames/s
   (proposed, as in #261; 43 930 in row 285) and no DMA or PWM errors.
2. `tone -s 3 > /scheme/audio` takes 3 s ± 0.1 s, and `audiod` is still
   running afterwards.
3. `tone check` passes.

### REQ-AUDIO-2 Audible on both channels (beta)

Feature issue: [#472](https://github.com/redox-rpi/redox-rpi/issues/472)

The sound shall be audible through headphones, with left and right on the
right sides.

| Field | Value |
|---|---|
| Rationale | A rate check alone passes with a silent or swapped output. |
| Priority | should |
| Kind | functional |
| Level | beta |
| PBS | 2.3.10 |
| Status | proposed |
| Since | v0.4.0 |
| Implemented by | #250 |
| Verification | test, person: checklist v0.4.0 test 17, row 280 (dev build, person), row 286 (rc2 visit, pass) |

Acceptance criteria:

1. A person hears `tone -s 3 > /scheme/audio` and a sweep on both sides.
2. `tone -c left -s 2 > /scheme/audio` is heard on the left only.
3. Any click at boot is noted in the report (not yet a pass criterion).

### REQ-AUDIO-3 Startup sound and volume (beta)

Feature issue: [#473](https://github.com/redox-rpi/redox-rpi/issues/473)

The Pi 3B+ shall play a short startup sound at boot, let root set and keep
the volume, and fade the output out at `reboot` and `poweroff`.

| Field | Value |
|---|---|
| Rationale | The startup sound shows that the system is up without a screen; a kept volume and no click make the jack usable. |
| Priority | should |
| Kind | functional |
| Level | beta |
| PBS | 2.3.10, 2.4.5 |
| Status | proposed |
| Since | v0.5.0 (proposed for v0.6.0) |
| Implemented by | #292, #336, PR #329, PR #354 |
| Verification | test, automated: row 414 (startup sound on 7 boots, `volume 70` kept over 5 reboots); QEMU pwm-audio check 26/26; test, person: rc1 person test (heard, no click at reboot) |

Acceptance criteria:

1. Each boot logs `startup-sound: played`. The median time from the
   kernel's first line to `redox login:` over 3 boots with the sound is at
   most 0.5 s (proposed) above the median over 3 boots of the same image
   with the sound off (serial log, no person needed).
2. `off` in `/etc/startup-sound` turns the sound off at the next boot.
3. `volume 70` sets the volume, `volume` prints 70, and the value is the
   same after `reboot`; `+10`, `-10` and `mute` work.
4. Goal, not yet a pass criterion: a person hears no click at `reboot`
   and `poweroff`. It becomes one after a listening test at a visit (rows
   315, 333) and, if needed, a fix (PR #354); no log shows a click, so it
   always needs a person. Until then REQ-AUDIO-2 criterion 3 stays a note.
   State: `reboot` "no click" in the v0.5.0-rc1 person test (to confirm
   at the next visit that it was listened to); `poweroff` not checked.

New for v0.6.0 (Requirements agent, 2026-11-16): written from the v0.5.0
release notes and row 414; criteria 1 and 4 as the audio owner proposed
(2026-11-16). Impact: release checklist (audio steps), the
nightly's `pi-test --extended` audio check, REQ-AUDIO-2 criterion 3.
