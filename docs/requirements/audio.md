# Audio (`AUDIO`)

Topic: [audio](../topics/audio.md). Trace: [trace.md](trace.md).

### REQ-AUDIO-1 Sound from the headphone jack (beta)

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
| Verification | test, automated: checklist v0.4.0 test 16, row 285 |

Acceptance criteria:

1. `cat /scheme/audiohw/stats` shows a rate within 2 % of 44 100 frames/s
   (proposed, as in #261; 43 930 in row 285) and no DMA or PWM errors.
2. `tone -s 3 > /scheme/audio` takes 3 s ± 0.1 s, and `audiod` is still
   running afterwards.
3. `tone check` passes.

### REQ-AUDIO-2 Audible on both channels (beta)

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
| Verification | test, person: checklist v0.4.0 test 17, row 280 (dev build, person), row 286 (queued) |

Acceptance criteria:

1. A person hears `tone -s 3 > /scheme/audio` and a sweep on both sides.
2. `tone -c left -s 2 > /scheme/audio` is heard on the left only.
3. Any click at boot is noted in the report (not yet a pass criterion).
