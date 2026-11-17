# Experiment: tuxtype, Tux Typing with German lessons

For [#529](https://github.com/redox-rpi/redox-rpi/issues/529).
[Tux Typing](https://github.com/tux4kids/tuxtype) 1.8.3 (GPL-2.0) is a
typing tutor for children. It comes with its German theme: German menus,
lessons (`basis_lektion_01` to `08`) and word lists. It is built from
source with SDL 1.2 and [t4kcommon](https://github.com/tux4kids/t4kcommon),
Tux4Kids' shared library. The recipes are in `recipes/games/tuxtype` and
`recipes/wip/games/math/t4kcommon` here.

It is its own experiment so that the release image stays small. Build it
after the desktop experiments, with the German keyboard layout:

```bash
scripts/build-image.sh --experiment timesync --experiment rpi-fb --experiment orbital \
    --experiment orbital-apps --experiment cosmic-apps-source --experiment desktop \
    --experiment keyboard-de --experiment tuxtype
```

- The German layout comes from experiment
  [`keyboard-de`](../keyboard-de/README.md) (PR #581): the full de map,
  AltGr and dead keys.
- Launcher entry `72_tuxtype`. The desktop user (`user`) gets
  `~/.config/tuxtype/settings.txt` with `lang=deutsch`, so Tux Typing starts
  in German. It saves its settings, the language too, in that file. By hand:
  `tuxtype --theme deutsch`, or with `--window` for a window instead of full
  screen.
- t4kcommon is built without speech (there is no espeak on Redox, so
  `t4k_tts.c` is a stub) and without SDL_Pango, librsvg and SDL_net. It is
  static and linked into `tuxtype`.
- It needs relibc patch 0020. Tux Typing reads its files line by line with
  `fscanf(f, "%[^\n]\n", ...)`. relibc's scanf skipped white space only
  for a space in the format, so it read the first line forever.
- **Size:** 25 MB (`/usr/share/tuxtype`: images 12 MB, sounds 4.6 MB,
  themes 4.4 MB, fonts 2.8 MB). Together with the desktop it does not fit
  the 256 MiB network-boot filesystem, so this experiment sets 384 MiB.
  The network-boot image runs from RAM. That is fine on the Pi 400 and the
  3B+. On the Zero 2 W (512 MB) use an SD-card build (`--sdcard`), which
  this experiment sets to 384 MiB as well.

[`qemu-tuxtype-check.sh`](qemu-tuxtype-check.sh) types the German layout
test at the console. Then it starts the desktop and Tux Typing and walks to
lesson 1 with the keyboard, taking screenshots.

Limits:

- The menu help lines stay English. relibc's `setlocale(LC_ALL,
  "de_DE.UTF-8")` returns NULL, so gettext does not use the German
  translation (`/usr/share/locale/de/LC_MESSAGES/tuxtype.mo`). The theme's
  own words (menu entries, lessons) are German.
- No music: SDL_mixer here has no Ogg support for `Mix_Init`
  (`Failed to init required ogg and mod support`).
