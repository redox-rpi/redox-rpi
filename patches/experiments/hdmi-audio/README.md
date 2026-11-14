# Experiment: hdmi-audio

Sound over HDMI on the Pi 3B+ and Zero 2 W, issue
[#423](https://github.com/redox-rpi/redox-rpi/issues/423). The Zero 2 W has
no headphone jack, so HDMI is its only sound output. The plan is in the
issue; the background is in [docs/topics/audio.md](../../../docs/topics/audio.md).

- **Base `0001`, `bcm2835-hdmiaudiod`.** A new driver in the initfs. It
  replaces `bcm2835-pwmaudiod` in `40_drivers.target`, so with this
  experiment the jack is silent. It programs only the audio part of the
  VC4 HDMI block, as Linux `vc4_hdmi.c` does. The firmware keeps the
  picture.
  - **MAI FIFO** (HD block `0x7e808000`): it holds 2 channels of PCM at
    44.1 kHz. The sample clock `MAI_SMP` comes from the HSM clock: the
    rate in `CM_HSMCTL`/`CM_HSMDIV` when the source is known, otherwise
    163.682864 MHz. Two 3 s measurements of the DMA progress that agree
    correct it once; a refresh gap over 60 ms starts the measurement over
    (row 417: missed ring turns had measured 12.6 % too slow).
  - **HDMI core** (`0x7e902000`):
    - channel map and mask
    - audio packets (B-frame identifier 8)
    - N = 128 × fs / 1000, and CTS from the pixel clock: the one
      `rpi-firmwared` reports, else PLLH_PIX from the clock manager (the
      3B+ firmware reports none), else 148.5 MHz
    - the audio info frame in packet RAM slot 4
    - DVI mode is switched to HDMI mode
  - **DMA channel 0, DREQ 17** (not channel 2, which the PWM driver has):
    a ring of 8 × 512 frames of IEC958 subframes, which carry the
    preamble, channel status and parity, into `MAI_DATA`.
  - **The 10 ms timer:**
    - follows the ring, as in the PWM driver
    - counts FIFO underflows and overflows
    - programs the audio part again when the firmware has reset it (on a
      hotplug)

  | Path | Read | Write |
  |---|---|---|
  | `/scheme/audiohw` | – | exactly 2048 bytes (512 frames, s16le, stereo, 44.1 kHz), as for the PWM driver |
  | `/scheme/audiohw/stats` | what the driver found (`hotplug`, `hdmi_mode`, `pixel_hz`, `hsm_hz` and its source), register values, `measured_rate_hz`, `infoframe_active`, `mai_underflow_polls`, ring and error counters | – |
  | `/scheme/audiohw/tone` | – | `<seconds> [<hz>]`: a sine through the ring, at a quarter of full scale |
  | `/scheme/audiohw/stop` | – | any write stops the FIFO and the info frame |

  Without an HDMI output (QEMU, or no display at boot) the driver says so
  and plays the ring to the clock.

`qemu-check.sh` checks the start, `audiod` on top and the scheme in QEMU.
It can't check the HDMI path, since QEMU has none.

## Tests on the Pi

1. **3B+ with the NanoKVM, automated.** `stats` after boot, `tone 3`, the
   chime, `stats` again. It passes when:
   - `mode hdmi-dma`, `hdmi_mode yes`, `infoframe_active yes`
   - `rate_window_hz` and `rate_last_measured_hz` within 1 % of 44100
   - `mai_underflow_polls` and `dma_faults` 0 while sound plays
   - `loud_chunks` grows
2. **Listening, with a person:** a monitor or TV with speakers, on the 3B+
   and the Zero 2 W: chime, `tone`, Sopwith.

Don't combine with `rpi-fb`: both change the `INITFS_BINS` line of the base
`Makefile`.
