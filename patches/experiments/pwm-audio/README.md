# Experiment: pwm-audio

**Default since PR #251 (v0.4.0, beta):** base patch `0086` (was `0001`
here), the `audiod` part (was `0002`) is in base `0030`, kernel `0050`
is a default kernel patch, and `tone` is in every image. The name
`pwm-audio` is still accepted and does nothing; `qemu-check.sh` stays
here and now checks the default image.

Sound from the headphone jack of the Pi 3B+, issue
[#250](https://github.com/redox-rpi/redox-rpi/issues/250). Plan and
background: [docs/topics/audio.md](../../../docs/topics/audio.md).

- **Base `0001`, `bcm2835-pwmaudiod`** (new driver in the initfs, after
  `bcm2835-gpiod`): PWM0/PWM1 on GPIO 40/41 (ALT0), clock PLLD/2 =
  250 MHz from the clock manager, range 2834 (44 107 Hz, about 11.5 bits:
  the FIFO gives one word per period to the two channels in turn, test
  #281, so a stereo frame takes two periods).
  DMA channel 2 (only if `brcm,dma-channel-mask` has it) plays a ring of
  8 × 512 frames into the PWM FIFO (DREQ 5); a 10 ms timer follows
  `CONBLK_AD`, fills played slots with silence and lets waiting writes
  in. The ring starts with a 46 ms ramp from 0 V to the middle.

  | Path | Read | Write |
  |---|---|---|
  | `/scheme/audiohw` | – | exactly 2048 bytes (512 frames, s16le, stereo, 44.1 kHz); waits, or `EWOULDBLOCK` non-blocking, while the ring is full |
  | `/scheme/audiohw/stats` | clock, range, measured rate, ring and error counters | – |
  | `/scheme/audiohw/tone` | – | `<seconds> [<hz>]`: a sine with the CPU filling the FIFO, no DMA (stage A1); result in `stats` |

  Without a PWM block (QEMU) the driver says so and plays the ring to the
  clock, so `audiod` and the scheme run in QEMU too.
- **Base `0002`:** `audiod` starts on `raspi3bp` again. At the merge this
  goes into base patch `0030` (drop its `audiod` condition) instead of a
  patch of its own.
- **Kernel `0050`:** the same patch as in experiment `rpi-fb` (PR #113):
  clean and invalidate the data cache over new DMA memory, so a stale
  cache line cannot be written back over the ring or its control blocks
  (docs/topics/dma.md). Do not combine `pwm-audio` with `rpi-fb`; the
  patch would apply twice. Whichever reaches `main` first makes it a
  default kernel patch.
- **Tool `tone`** (`tools/tone`, in the image through `config.toml`):

  ```
  tone -s 3 > /scheme/audio          1 kHz sine, 3 s, through audiod
  tone -c left -f 440 -o /tmp/l.raw  left channel only, into a file
  tone sweep -s 5 > /scheme/audio    100 Hz to 10 kHz
  tone check                         audiohw contract (EINVAL, EWOULDBLOCK)
  ```

Patch numbers here are placeholders; the merge gives the real ones.

## QEMU

[`qemu-check.sh`](qemu-check.sh): QEMU's `raspi3b` keeps the PWM
registers but has no PWM clock (`CM_PWMCTL` never shows BUSY), so the
driver says `PWM clock does not start ... (QEMU?), playing to the clock`
and counts slots as played by the timer. Its DMA model would also run the
endless ring at once, so the ring is never started there. The check covers
the start-up, `audiod` on top, the `audiohw` contract, `tone` and `cat`
through `/scheme/audio`, and the stats; not the sound.

## Results

- 2026-11-07, QEMU, `v0.4.0-dev-704-gfa8f2ef9+x6d83`: 15 of 15 checks
  pass. Short write `EINVAL`, full ring `EWOULDBLOCK` (audiod keeps it
  full), 44 082–44 091 frames/s measured, 1 underrun (the end of the
  first sound), `tone > /scheme/audio` and `cat file > /scheme/audio`
  exit 0.
- 2026-11-07, Pi #279, `v0.4.0-dev-704-gfa8f2ef9+x6d83` (range 5669):
  **fail, half rate.** Everything ran without faults, but 22 050 frames/s
  (DMA ring and CPU tone alike).
- 2026-11-07, Pi #281, diagnosis with the tone options: the clock is right
  (250 MHz); the FIFO gives one word per period to the two channels in
  turn (stereo 44 099 words/s at range 5669, mono 43 515; range 2834
  doubles both, `div=4` halves both). So range = clock / (2 × rate).
- 2026-11-07, Pi #283, `v0.4.0-dev-707-g7c92bc80+x6d83` (range 2834):
  **pass.** Boot line right; `measured_rate_hz` 44 111 at 22 s and 53 s,
  `dma_faults 0`, `pwm_errors 0` (gap bits about 44 polls/s, expected in
  stereo); CPU tone 3.00 s, 44 053 frames/s; `tone check` good;
  `tone -s 5` 4.96 s, `tone -s 30` 30.2 s through `audiod`. CPU time
  during 30 s of sound: `bcm2835-pwmaudiod` about 0.35 s (about 1 % of a
  core), `audiod` about 0.8 s over its two threads (about 3 %), part of
  it spent mixing silence all the time. `pi-test.py` standard checks pass.
- 2026-11-07, Pi #284, default build `v0.4.0-dev-730-gaa76c1f9`: **pass**
  (44 111–44 125 frames/s, no faults, `pi-test.py` passes).
- 2026-11-07, Pi #280, listening with headphones, same build: **pass.**
  All tones, left only in the left ear, the sweep and volume 100 fine;
  faint hiss at idle, a little crackle (cheap headphones?); the boot click
  was not checked. 44 minutes without a DMA fault or PWM error.

