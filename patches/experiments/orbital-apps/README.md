# Experiment: orbital-apps, a few desktop apps for Orbital (#33)

On top of experiment [`orbital`](../orbital/README.md): a text editor and
a file manager from upstream's `config/desktop.toml`, taken as
binary aarch64 packages from `static.redox-os.org` like the rest of the
image.

```bash
scripts/build-image.sh --experiment timesync --experiment rpi-fb \
    --experiment rpi-fb-dma-on --experiment orbital --experiment usb-mouse \
    --experiment orbital-apps
```

- [`config.toml`](config.toml): `cosmic-edit`, `cosmic-files`,
  `cosmic-icons`, `hicolor-icon-theme`. Each app puts its own entry in
  `/usr/share/ui/apps/`, so the launcher shows it. `cosmic-term` is left
  out; `orbterm` stays the terminal.

## Sizes

Installed size (`storage_size` in the package index, 2026-11-02), with the
dependencies the image does not have yet:

| app | package | with new dependencies | biggest dependencies |
|---|---|---|---|
| NetSurf 3.11 (browser) | 3.8 MB | 57.4 MB | openssl3 24.5, liborbital 9.7, libjpeg 4.3, curl 3.5 |
| COSMIC Text Editor 1.10 | 27.1 MB | 48.5 MB | libxml2 7.2, xkeyboard-config 7.0, libxcb 5.5 |
| COSMIC Files 1.11 | 28.2 MB | 28.2 MB more | shares the editor's |
| cosmic-icons, hicolor-icon-theme | 0.9 MB | 0.9 MB | |
| **all four** | | **135 MB** | |
| **in the image (without NetSurf)** | | **78 MB** | |

Much of it is static libraries and headers in the library packages (for
example openssl3), which the binary packages carry along.

## Results

QEMU `raspi3b`, 2026-11-02, apps started from the serial root shell over
Orbital's login screen (the Orbital login does not get through in QEMU):

- `v0.4.0-dev-70-g764932b+x6d96` (with NetSurf, netboot image, 256 MB
  filesystem): `df` showed 213.5 of 222 MB used, 8.5 MB free. Too tight
  for a desktop, one reason NetSurf left again.
- **COSMIC Text Editor** and **COSMIC Files** open their windows and draw
  text and icons (fonts from the known font paths). `ps`: `cosmic-edit`
  53 MB, `cosmic-files` 59 MB private memory. With both open, `free`
  showed 320 MB used of 965 MB.
- **NetSurf** (`netsurf-fb`) stops at once, from the serial shell and
  from `orbterm` alike: `RELIBC PANIC: panicked at
  src/header/pthread/tls.rs:42`, `RefCell already borrowed`. That is
  `pthread_getspecific` finding its thread-local map already borrowed, in
  the relibc linked into the upstream package. Not yet checked whether it
  also happens on an image without this project's kernel patches, or on
  x86.
- `v0.4.0-dev-71-gfcbf1d5+x6d96` (without NetSurf): `df` 176.1 of 222 MB
  used, 45.9 MB free on the netboot image, so its 256 MB size stays.
  `/usr/share/ui/apps` has `10_terminal`, `20_cosmic-files`,
  `30_cosmic-edit` and `60_viewer`: the launcher shows the terminal,
  the file manager, the editor and the image viewer. Both COSMIC apps
  open again; `free` 314 MB used of 965 MB with both open.

## Tux Typing (#529)

[Tux Typing](https://github.com/tux4kids/tuxtype) 1.8.3 (GPL-2.0), a
typing tutor for children, with its German theme: German menus, lessons
(`basis_lektion_01` to `08`) and word lists. Built from source with SDL 1.2
and [t4kcommon](https://github.com/tux4kids/t4kcommon), Tux4Kids' shared
library (recipes in `recipes/games/tuxtype` and
`recipes/wip/games/math/t4kcommon` here). For German keyboards add
experiment [`keymap-de`](../keymap-de/README.md).

- Launcher entry `72_tuxtype`. The desktop user (`user`) gets
  `~/.config/tuxtype/settings.txt` with `lang=deutsch`, so it starts in
  German; tuxtype saves its settings, the language too, in that file. By hand:
  `tuxtype --theme deutsch` (`--window` for a window instead of full
  screen).
- t4kcommon without speech (no espeak on Redox: a stub `t4k_tts.c`),
  without SDL_Pango, librsvg and SDL_net; static, linked into `tuxtype`.
- Needs relibc patch 0020: tuxtype reads its files line by line with
  `fscanf(f, "%[^\n]\n", ...)`, and relibc's scanf only skipped white
  space for a space in the format, so it read the first line forever.
- Size: 25 MB (`/usr/share/tuxtype`: images 12 MB, sounds 4.6 MB, themes
  4.4 MB, fonts 2.8 MB). Together with experiment `desktop` it does not
  fit the 256 MiB netboot filesystem; the QEMU check ran with a 384 MiB
  filesystem.

[`qemu-tuxtype-check.sh`](qemu-tuxtype-check.sh) types the German layout
test at the console, then starts the desktop and tuxtype and walks to
lesson 1 with the keyboard, taking screenshots.

Limits:

- The menu help lines stay English: relibc's `setlocale(LC_ALL,
  "de_DE.UTF-8")` returns NULL, so gettext does not use the German
  translation (`/usr/share/locale/de/LC_MESSAGES/tuxtype.mo`). The theme's
  own words (menu entries, lessons) are German.
- No music: SDL_mixer here has no Ogg support for `Mix_Init`
  (`Failed to init required ogg and mod support`). Whether the WAV sound
  effects play is not checked yet: the audio counter stayed the same in
  the QEMU check, which never got into a game.
