# Audio

Sound on the Pi 3B+: what Redox already has, how the Pi makes sound, and
a plan that starts with the headphone jack through PWM. Written
2026-11-07 before any work started. The PWM driver is
[#250](https://github.com/redox-rpi/redox-rpi/issues/250), planned for
**v0.4.0 as a beta** (maintainer, 2026-11-07): A1 and A2 tested on the Pi
are enough; A3 and polish may follow in a later release.

## State

- **No sound on `main` yet.** Our base patch `0030` leaves `audiod` out
  on `raspi3bp` (`condition_board_not = ["raspi3bp"]` in
  `init.d/20_audiod.service`), so the boot shows no audio errors (#18).
- **In progress:** the PWM driver `bcm2835-pwmaudiod`, stages A1 and A2
  ([#250](https://github.com/redox-rpi/redox-rpi/issues/250), experiment
  [`pwm-audio`](https://github.com/redox-rpi/redox-rpi/tree/pwm-audio/patches/experiments/pwm-audio),
  draft PR #251). It passes the QEMU check (driver up, `audiod` on top,
  `EINVAL`/`EWOULDBLOCK` contract, `cat file > /scheme/audio`); QEMU has
  no PWM clock, so there it plays to the clock instead of the jack. Pi
  test #279 (driver, CPU tone, DMA ring, no listening) is queued; the
  listening test #280 needs a person with headphones.
- **Release plan:** v0.4.0, beta. Once #279 and #280 pass, PR #251 moves
  the patches to the default directories, so the driver and `audiod` run
  in every image, and the v0.4.0 release notes list sound from the jack
  as **beta** (known limits: about 11 usable bits, one fixed format, no
  stop after silence yet, left/right as found in #280). If #279 shows
  problems that could hurt the boot, it ships as the experiment
  `pwm-audio` instead, also marked beta.
- **The hardware is there.** The 3.5 mm jack is fed by the SoC's PWM
  block; HDMI can carry sound; the GPIO header has an I2S port for DAC
  boards.

## What Redox has

Redox has a small audio stack in `base`, made of two layers that talk
through schemes:

```
program ──write──▶ /scheme/audio ──▶ audiod (mixer) ──write──▶ /scheme/audiohw ──▶ hardware driver
                   (one handle per sound)          (512-frame chunks)
```

- **`audiod`** (`base/audiod`, about 300 lines) offers `/scheme/audio`.
  Every program that opens it gets its own buffer; `audiod` mixes all of
  them into chunks of 512 stereo frames and writes each chunk to
  `/scheme/audiohw`. It also offers `/scheme/audio/volume` (0–100; the
  volume is applied as a cube, to follow how loudness is heard). After
  start-up it drops into a namespace with only `memory` and `rand`.
- **The format is fixed:** signed 16-bit, little endian, two channels,
  44 100 Hz. Nothing in the scheme says so; it is what the drivers set up
  and what programs write. There is no resampling and no format
  negotiation.
- **A hardware driver** registers the scheme `audiohw` and takes writes of
  exactly 2048 bytes (512 frames × 4 bytes). When its queue is full it
  answers `EWOULDBLOCK` and `audiod` waits. A driver may also offer
  `audio_shm`, a shared-memory ring (`redox_rings`), so the chunks are not
  copied through a syscall (`ihdad` does; `audiod` falls back to plain
  writes otherwise).
- **Three drivers, all for PCs:**

| Driver | Hardware | Size |
|---|---|---|
| `ihdad` | Intel High Definition Audio (most PCs since 2004) | about 2 500 lines |
| `ac97d` | AC'97 (older PCs, QEMU `-device AC97`) | about 460 lines |
| `sb16d` | Sound Blaster 16 (ISA, QEMU `-device sb16`) | about 350 lines |

  All three are PCI or ISA devices started by `pcid`, with a ring of
  buffers that the sound chip reads by itself (bus master DMA). `ac97d`
  is the closest model for a Pi driver: one output, 32 sub-buffers of
  2048 bytes, an interrupt when one has been played.
- **Programs:** anything that writes raw samples to `/scheme/audio`, for
  example the Redox ports of SDL, which have an audio backend for it.
  `cat sound.raw > /scheme/audio` plays a raw file.

So a Pi driver only has to provide `audiohw`; the mixer, the volume and
the programs stay as they are.

## How the Pi makes sound

### The headphone jack: PWM

The 3B+ has no sound chip for the jack. Two PWM channels of the SoC make
the left and right signal, and a filter on the board turns them into an
analogue signal:

```
samples in RAM ─▶ DMA controller ─▶ PWM FIFO ─▶ PWM0 / PWM1 ─▶ GPIO 40 / 41 ─▶ RC low-pass + buffer ─▶ jack
                  (DREQ 5)                       (clock from the clock manager)
```

- **PWM:** each output switches between 0 V and 3.3 V many times per
  sample; the share of time it is high (the duty cycle) is the sample
  value. The low-pass filter on the board averages that into a voltage.
- **Registers** (BCM2835 ARM Peripherals, chapter 9; the 3B+ is the same):
  the PWM block at bus `0x7E20_C000` (ARM `0x3F20_C000`). `CTL` turns the
  channels on and selects the FIFO and mark-space mode, `RNG1`/`RNG2` set
  the range (the number of clock ticks per sample period), `FIF1` is the
  FIFO both channels read from (left and right words in turn), `STA`
  shows FIFO full or empty and errors, and `DMAC` turns on the DMA
  request with its thresholds.
- **Pins:** GPIO 40 (PWM0) and 41 (PWM1), function ALT0. They go only to
  the jack, not to the 40-pin header. `bcm2835-gpiod` already sets pin
  functions.
- **Clock:** the PWM clock comes from the clock manager (`CM_PWMCTL` and
  `CM_PWMDIV`, written with the password `0x5A`), with PLLD (500 MHz on
  the Pi 3) as its source. The clock manager is not in the datasheet's
  PWM chapter; its registers are documented by the community and in the
  devicetree bindings.
- **Resolution:** the range is clock ÷ sample rate. At 250 MHz (PLLD
  divided by 2) and 44.1 kHz that is about 5 670 steps, about 12.5 bits.
  The analogue part adds noise, so expect about 11 usable bits: fine for
  system sounds and speech, average for music. Oversampling with noise
  shaping can improve it later, at a higher CPU cost.
- **The DMA controller** feeds the FIFO: a ring of control blocks that
  points back to its start, paced by the PWM's DMA request (`PERMAP` 5).
  The driver refills one half of the buffer while the other half plays.
  Our [DMA plan](dma.md) covers the controller; audio is its third user
  after sdhost (channel 4) and the framebuffer blits (channel 5). The
  plan keeps channels 2 and 0 free "for SPI or audio".
- **Pops:** the outputs rest at 0 V, playing silence means half the
  range (1.65 V). Jumping between the two clicks in the headphones, so
  the driver ramps slowly up to the middle when it starts and down when
  it stops.
- **The fourth ring of the jack (TRRS)** carries composite video from the
  VideoCore's video encoder, not sound. Not part of this plan.

The firmware does not touch the PWM block unless Linux asks it to play
sound through VCHIQ, so a driver of ours owns it fully. It also works
with our own [open firmware](open-firmware.md), which brings no audio
service at all.

### HDMI audio

The HDMI output can carry sound next to the picture. Linux has two ways:

- `bcm2835-audio` sends samples over **VCHIQ** to the closed firmware,
  which plays them on HDMI or the jack. This is what the old wish list
  meant.
- The Linux `vc4` display driver plays HDMI sound **itself**: the samples
  go by DMA into the HDMI block's audio FIFO (MAI), and the driver sets the
  audio info frames. That needs a driver that owns HDMI: the later part of
  stage (a) of [`vc4d`](graphics.md), which takes over the pixel valves
  and HDMI (today `vc4d` takes over the HVS only; HDMI timing stays with
  the firmware).

### I2S on the GPIO header

The PCM/I2S block (bus `0x7E20_3000`) sends digital audio on GPIO 18–21
(header pins 12, 35, 38, 40) to a DAC board. A board with a PCM5102A chip
(a few euros) needs no set-up over I2C; HiFiBerry-style boards with a
PCM5122 also need a few I2C writes (`bcm2835-i2cd` exists). Same DMA
pattern as PWM (`PERMAP` 2 for transmit). Best sound quality of all ways,
with little code.

### USB sound cards

Class-compliant USB sound cards (UAC 1) work on every OS without a vendor
driver. They need isochronous transfers in `dwc2d`, and they share the
one USB 2.0 port with Ethernet and the other USB devices.

## Ways compared

| Way | Needs | Depends on the closed firmware | Quality | Size |
|---|---|---|---|---|
| **PWM, headphone jack** | driver for clock, PWM, DMA; `audiod` on | no | about 11 bits | small to medium |
| I2S DAC board | driver for PCM/I2S and DMA, optional I2C | no | 16–24 bits | small to medium |
| HDMI through `vc4d` | `vc4d` owning HDMI, MAI FIFO, info frames | no | 16–24 bits | medium, after `vc4d` |
| USB sound card | isochronous transfers in `dwc2d`, USB audio class | no | 16–24 bits | large |
| VCHIQ, like Linux | VCHIQ protocol and its audio service | **yes** | jack and HDMI | large |

PWM comes first: it needs no extra hardware, every 3B+ has the jack, the
driver is small, and it builds on pieces we have (`bcm2835-gpiod`, the DMA
plan). I2S is the natural second step, with the same DMA code. VCHIQ is
no longer needed for sound; it stays on the wish list for the camera and
the video decoder.

## Plan: PWM driver

A new driver `bcm2835-pwmaudiod` in our base patches, started from the
devicetree (`/soc/pwm@7e20c000`), offering `audiohw`.

1. **A1, tone without DMA (a few days).** Set GPIO 40/41 to ALT0, start
   the PWM clock, set the range, and let the CPU write a 1 kHz sine into
   the FIFO while `STA` says it has room. Proves pins, clock and PWM. The
   CPU is busy all the time; that is fine for a first test.
2. **A2, `audiohw` with DMA (1–2 weeks).** A ring of control blocks over
   an uncached `Dma` buffer (step 1 of the DMA plan is enough: the driver
   owns the buffer, and 44.1 kHz stereo is only about 350 KB/s of
   writes). Take channel 2 if it is in the devicetree mask. Convert each
   512-frame chunk from `audiod` into PWM values; answer `EWOULDBLOCK`
   when the ring is full; an interrupt (or polling `CS`) when a half has
   played. Turn `audiod` back on for `raspi3bp` (drop the condition from
   patch `0030`). Then `cat sound.raw > /scheme/audio` plays.
3. **A3, behaviour (a few days).** Ramp against pops, silence on underrun
   instead of repeating old data, stop the PWM after a while without
   sound, a short report in the boot log (clock, range, channel). Measure
   the CPU time while playing.
4. **A4, later.** Oversampling with noise shaping; other sample rates;
   the `audio_shm` ring; then I2S on top of the same DMA module.

How A1 and A2 came out (experiment `pwm-audio`, #250):

- **One driver for both stages.** A1 is the file `/scheme/audiohw/tone`
  (`echo 3 > /scheme/audiohw/tone`): the driver holds the DMA ring and
  writes a 1 kHz sine into the FIFO with the CPU, and reports how many
  frames per second the PWM took. That measures the clock and the range
  without anyone listening.
- **A2:** 8 slots of 512 frames (about 93 ms), one control block per slot,
  the last pointing back to the first. No interrupt: a 10 ms timer reads
  `CONBLK_AD`, fills each played slot with silence and retries writes
  waiting for room. So two parts of A3 are already in: an underrun plays
  silence, and the ring starts with a 46 ms ramp from 0 V to the middle.
  `/scheme/audiohw/stats` has the measured rate, underruns and error
  counters.
- **Kernel patch `0050`** (cache clean over new DMA memory, from `rpi-fb`)
  is in the experiment too; the DMA plan wants it with the first DMA
  driver on `main`.
- **QEMU** keeps the PWM registers but has no PWM clock, and its DMA model
  would run an endless ring at once. The driver detects the missing clock
  and plays the ring to the timer instead, which is enough to test
  `audiod` and the scheme.
- **Tool `tone`** writes test sound (sine, sweep, one channel) for
  `/scheme/audio`. Rust's `stdout` is line buffered and splits binary data
  at `0x0a` bytes; `audiod` takes whole frames only, so the tool writes
  to file descriptor 1 directly.

Rules: hardware documentation (the BCM2835 ARM Peripherals datasheet and
its errata, the devicetree bindings) is fine to use. Code from GPL drivers
(Linux `pwm-bcm2835`, `bcm2835-audio`, `vc4_hdmi`) and from GPL bare-metal
libraries (Circle) is not.

## Testing

Nobody can hear the Pi from the test rack, and the NanoKVM captures only
HDMI. Two ways:

- **Automatic (proposed):** a cheap USB sound card with a line input on
  homenodellm, a cable from the Pi's jack to it. The test plays a known
  tone or sweep on the Pi, records it on homenodellm, and checks the
  frequency, the level and the noise with a small script. Needs about
  10 euros of hardware and a cable, plugged in once by the maintainer.
- **By ear:** a person with headphones, bundled with other person tests
  ([prepare person tests](../test-plan.md)).

In QEMU `raspi3b` the PWM block is not emulated, so only the driver's
start-up and the `audiohw` scheme can be checked there; the sound needs
the Pi.

## Other Pi models

- **Zero 2 W:** no jack. PWM still works on header pins (GPIO 12/13 or
  18/19, ALT0/ALT5) with an external filter; I2S works the same as on the
  3B+.
- **Pi 4:** the same PWM jack on GPIO 40/41 and the same blocks, at the
  Pi 4's peripheral addresses (from the devicetree).
- **Pi 5:** no jack at all. Sound goes through HDMI or I2S in the RP1 chip,
  a new driver.

## Later

- HDMI audio through `vc4d`, once it owns HDMI.
- USB sound cards, once `dwc2d` has isochronous transfers.
- Input: a microphone through I2S or USB (`audiod` has no input side yet,
  that would be upstream work).
