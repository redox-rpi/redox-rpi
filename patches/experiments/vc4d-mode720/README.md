# vc4d-mode720

Real mode change to 1280x720 at 60 Hz (#563, step 2 of the picture modes).

vc4d reads `/etc/vc4d-mode`. If the monitor's EDID lists 1280x720 and the
pixel clock of the firmware's mode is an integer multiple of it (1080p60:
148.5 MHz, 720p60: 74.25 MHz), vc4d stops pixel valve 2, multiplies the
PLLH_PIX channel divider, writes the HDMI and valve timing, starts the valve
and sets the HVS channel size. The monitor resyncs once (a short black
screen). Serial shows `vc4d: mode: switch: ... PASS`.

If the mode is not reachable, vc4d scales it as in `vc4d-scale`. The file
content `1280x720 scale` forces scaling.

Build: `--experiment vc4d --experiment vc4d-flip ... --experiment vc4d-mode720`.
QEMU has no HDMI: there vc4d scales, so QEMU only checks that nothing broke.
