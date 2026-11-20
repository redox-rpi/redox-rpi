# audiotop

A live view of the audio driver's counters, for #565. It reads `/scheme/audiohw/stats` every interval and prints one line per interval: the rate and how much each counter grew. It works with `bcm2835-hdmiaudiod` (HDMI) and with `bcm2835-pwmaudiod` (headphone jack).

```text
audiotop [-i seconds] [-n rounds] [-c file.csv] [-f stats-file] [-q]
```

- `-i`: the interval in seconds (default 1).
- `-n`: how many rounds (default 0 = until Ctrl-C).
- `-c`: write every numeric stats key into a CSV, one row per interval.
- `-q`: print only the summary.

Example, during a 10 s tone:

```text
audiotop -i 2 -n 6 -c /tmp/audio.csv & tone -s 10 > /scheme/audio
```

Line markers:

- `!` and a name: an error counter grew (`dma_faults`, `mai_underflow_polls`, `mai_overflow_polls`, `pwm_errors`).
- `~` and a name: a warning counter grew (`underruns`, `long_gaps`, `pwm_gap_polls`). `long_gaps` grows by one when sound starts after silence.

Exit codes, for pi-tests rows:

- 0: no error counter grew, and at the end the rate (`rate_window_hz`, else `measured_rate_hz`) is within 2 % of `sample_rate`, as REQ-AUDIO-4 (#539) asks.
- 1: an error counter grew.
- 2: the rate is off or missing.
- 3: the stats file cannot be read.

Host tests: `cargo test`.
