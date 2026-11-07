# Experiment: desktop, an optional desktop started from the console

For the release image (v0.4.0-rc2, [#33](https://github.com/redox-rpi/redox-rpi/issues/33)):
Redox's desktop, Orbital, with the COSMIC text editor and file manager is
in the image but does not start at boot. The Pi boots to the text console
as without it. As root:

    desktop

starts Orbital on VT 3 and switches to it: the login window (user `user`,
empty password: press Enter), then the desktop with the launcher. Keys:

- **Super+F2**: back to the text console (Orbital keeps running);
- **Super+F3**: to the desktop again (or run `desktop` again).

Orbital runs until the next reboot; its log goes to `/tmp/desktop.log`.

Build it after the experiments it configures:

    --experiment rpi-fb --experiment orbital --experiment orbital-apps \
    --experiment cosmic-apps-source --experiment desktop

- `rpi-fb`: the display through the firmware (`rpi-fbd`), double
  buffered; not `vc4d`, which is still an experiment (noise bands on HDMI
  at Pi test #275);
- `orbital`: Orbital, orbterm, the light desktop; this experiment turns
  off its start at boot (`20_orbital`) and puts the switch to the console
  (`30_console_switch`) back;
- `orbital-apps`: COSMIC Text Editor and COSMIC Files with their icons;
- `cosmic-apps-source`: both apps built from source with the icon lookup
  fix (Pi test #274: the editor no longer keeps redoxfs busy, the save
  dialog appears);
- USB mouse and keyboard are default patches.

`app-log` (app stderr and exit status on serial) stays a test experiment.

## Size

The desktop makes the image larger: the SD card image of the desktop
visit images was about 48 MB as `.img.xz`, against about 10 MB without
the desktop. Measured for the release build in the Results below.

## Beta limits (for the release notes)

- The text editor is slow to start (about 40 s for the first window on
  the Pi 3B+) and costs about half a core or more while typing: it
  redraws its whole window for every key (iced/libcosmic damage covers
  the whole window, see experiment `present-damage` on `graphics-vc4d`).
- The save dialog of the editor (it is COSMIC Files) appears after up to
  30 s, and in one visit (#275) not at all.
- Dragging a window costs about 90 % of a core: Orbital redraws in
  software.
- No browser and no cosmic-term (orbterm is the terminal, Super+T).
- One resolution: the firmware's mode at power-on. No HDMI hotplug.
- Orbital cannot be stopped from the console; reboot to free its memory.

## Results
