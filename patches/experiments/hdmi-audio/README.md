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

- **Base `0002`, the BCM2711 (Pi 4, Pi 400), issue
  [#543](https://github.com/redox-rpi/redox-rpi/issues/543).** The same
  driver also handles the two "vc5" HDMI ports. The Pi 400 has no jack, so
  HDMI is its only sound output.
  - **Port choice:** the port the firmware sends video on (`VID_CTL` bit
    31). With no screen, `hdmi_force_hotplug=1` + `hdmi_drive=2` in
    `config.txt` makes the firmware drive HDMI0 (Test 494). A port whose
    DVP reset is held or whose clock is stopped is never read, because its
    registers can hang the bus. QEMU raspi4b aborts any read of the DVP
    block, so a child process (`bcm2835-hdmiaudiod --probe-dvp`) reads it
    first; when the child dies, the driver plays to the clock.
  - **Registers** from Linux `vc5_hdmi_hdmi0_fields`/`_hdmi1_fields`:
    - MAI per port in the shared HD block (`0x7ef20000`)
    - the core at `0x7ef00700` or `0x7ef05700`
    - packet RAM in its own block
    - 4-bit channel map
    - MAI thresholds 0x10/0x1c
    - DREQ 10 for HDMI0 and 17 for HDMI1, from the low 5 bits of the node's
      `dmas` cell (the Pi devicetrees put flags above them)
  - **Clocks:** the sample clock comes from the fixed 108 MHz audio clock,
    and the measurement correction stays as on the 3B+. The pixel clock
    comes from the port's PHY PLL (`RM_OFFSET`, `TX_PHY_CLK_DIV`), because
    the Pi 400 firmware reports none. The PHY's random generator is powered
    up, as in Linux `vc5_hdmi_phy_rng_enable`.
  - **Devicetree:** addresses go through `/soc`'s `ranges` and
    `dma-ranges` with their real cell counts. The DMA ring must be in the
    first 1 GB of RAM.
  - **`stats` adds:**
    - `port` (`vc4`, `hdmi0`, `hdmi1`)
    - `video_now`
    - `mai_thr`, `mai_fmt`, `channel_map`
    - the DVP and PHY registers, where the 3B+ has the PLLH and HSM lines
  - **Applying with `pi400`:** `0001` applies with fuzz 1 on top of it,
    because `pi400` adds `40_bcmgenetd.service` next to the line `0001`
    changes in `40_drivers.target`.

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

3. **Pi 400, automated, no screen** (#543 step 3): `hdmi_force_hotplug=1` +
   `hdmi_drive=2`, images with `pi400` and `hdmi-audio`. `stats` after boot,
   `time tone -s 3 > /scheme/audio`, the chime, `stats` again. It passes when:
   - `port hdmi0`, `mode hdmi-dma`, `infoframe_active yes`, `pixel_hz
     25200000 phy`
   - the rate is within 2 % of 44100, as REQ-AUDIO-4 (#539) asks
   - `mai_underflow_polls` and `dma_faults` are 0
   - `loud_chunks` grows
   - `tone` takes 3 s ± 0.1 s
4. **Pi 400, listening** (person): a monitor with speakers. Chime, `tone`,
   `volume`, Sopwith.

Don't combine with `rpi-fb`: both change the `INITFS_BINS` line of the base
`Makefile`.
