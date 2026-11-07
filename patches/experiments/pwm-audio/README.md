# Experiment: pwm-audio

Sound from the headphone jack of the Pi 3B+, issue
[#250](https://github.com/redox-rpi/redox-rpi/issues/250). Plan and
background: [docs/topics/audio.md](../../../docs/topics/audio.md).

- **Base `0001`, `bcm2835-pwmaudiod`** (new driver in the initfs, after
  `bcm2835-gpiod`): PWM0/PWM1 on GPIO 40/41 (ALT0), clock PLLD/2 =
  250 MHz from the clock manager, range 5669 (44 098 Hz, about 12.5 bits).
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

## Results

None yet.
