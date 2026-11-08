# Experiment: desktop, an optional desktop started from the console

For the release image (v0.4.0-rc2, [#33](https://github.com/redox-rpi/redox-rpi/issues/33)):
Redox's desktop, Orbital, with the COSMIC text editor and file manager is
in the image but does not start at boot. The Pi boots to the text console
as without it. As root:

    desktop

first prints a warning: the desktop is a beta, with its main limits (see
Beta limits below), and asks `Start the desktop? [y/N]`. Only `y` or
`yes` starts it; any other answer, or no terminal on stdin (a script, a
pipe), leaves it off. Then it starts Orbital on VT 3 and switches to it:
the login window (user `user`, empty password: press Enter), then the
desktop with the launcher. When Orbital already runs, `desktop` switches
to it without asking. `desktop --yes` skips the question, for tests and
scripts (the Pi tests run it over devsh, which has no terminal). Keys:

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

The SD card image stays 1054 MiB (the partition size is fixed), but it
compresses less well: v0.4.0-rc1 is 20.9 MiB as `.img.xz`, the same
build with the desktop experiments 48.6 MiB (+28 MiB, about 2.3 times).
Most of it is the COSMIC editor and files apps and their icons and fonts.

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

- 2026-11-08, v0.4.0-dev-756-g5697fc1d+xce4c-dirty (QEMU, `--allow-dirty`
  for the known base 0083 fuzz, PR #254): boots to the text console;
  `desktop` starts Orbital on VT 3, login works; Super+F2 goes back to the
  console, `desktop` again switches to the same Orbital (one process in
  `ps`). SD image 48.6 MiB as `.img.xz` (rc1: 20.9 MiB). Pi test: row 287.
- 2026-11-08, Pi rows 287 A and 288 (same build): boots to the console,
  `pi-test.py` passes, `desktop` starts Orbital (rc 0, empty log), login
  with one Enter starts `launcher` as `user`, Super+F2 and `desktop` again
  keep one Orbital. Memory used went up by 40 to 80 MB. Not checked on
  screen (no screenshot tool for `rpi-fbd` yet).
- 2026-11-08, v0.4.0-dev-783-g0f83d540+xce4c (QEMU, clean build: main
  merged, no `--allow-dirty`), the warning: answer `n` starts nothing;
  `echo y | desktop` and `desktop < /dev/null` refuse (no terminal);
  `yes` starts Orbital; `desktop` again switches without asking, one
  Orbital; `desktop --yes < /dev/null` starts it; `desktop --bogus`
  prints the usage (rc 2). Ion's builtin `test -t` calls a pipe a
  terminal, so the script uses uutils' `/usr/bin/test`.
