# Experiment: desktop, an optional desktop started from the console

For the release image (v0.4.0-rc2, [#33](https://github.com/redox-rpi/redox-rpi/issues/33)):
Redox's desktop, Orbital, with COSMIC Files and the game SDL Sopwith is
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
After each login on the console `/etc/motd` points to it: "Type `desktop` to
start the graphical desktop (beta)".

Build it after the experiments it configures:

    --experiment rpi-fb --experiment orbital --experiment orbital-apps \
    --experiment cosmic-apps-source --experiment desktop

- `rpi-fb`: the display through the firmware (`rpi-fbd`), double
  buffered; not `vc4d`, which is still an experiment (noise bands on HDMI
  at Pi test #275);
- `orbital`: Orbital, orbterm, the light desktop; this experiment turns
  off its start at boot (`20_orbital`) and puts the switch to the console
  (`30_console_switch`) back;
- `orbital-apps`: COSMIC Files with its icons, and SDL Sopwith (a
  biplane game, keyboard: arrows, space fires, b bombs; GPL-2.0, an
  upstream recipe) in place of COSMIC Text Editor (since v0.4.1);
  Sopwith needs SDL2, which this experiment builds without OpenGL, see
  its `recipes/libs/sdl2/recipe.toml`;
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

- No text editor since v0.4.1: COSMIC Text Editor took 40 s to start
  on early builds (about 7 s on v0.4.0-rc2), half a core or more while
  typing (it redraws its whole window
  per key), and its save dialog did not open (#275, and the person test
  of 2026-11-08). SDL Sopwith takes its place.
- COSMIC Files is slow to open (up to 30 s).
- Sopwith draws in software (SDL2 without OpenGL); its window is about
  half the screen, Alt+Enter switches to full screen.
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
- 2026-11-08, desktop-game v0.4.0-dev-791-ga790a9d0+xce4c (QEMU): SDL2
  without OpenGL, Mesa and llvm21 left out, the netboot image fits in
  256 MiB again; `sopwith` opens its title screen in an Orbital window
  (320x200 at QEMU's 640x480).
