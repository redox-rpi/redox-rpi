# Experiment: tuxtype, Tux Typing with German lessons

For [#529](https://github.com/redox-rpi/redox-rpi/issues/529).
[Tux Typing](https://github.com/tux4kids/tuxtype) 1.8.3 (GPL-2.0) is a
typing tutor for children. It comes with its German theme: German menus,
lessons (`basis_lektion_01` to `08`) and word lists. It is built from
source with SDL 1.2 and [t4kcommon](https://github.com/tux4kids/t4kcommon),
Tux4Kids' shared library. The recipes are in `recipes/games/tuxtype` and
`recipes/wip/games/math/t4kcommon` here.

Tux Typing is not part of an image. It is a package in our package
repository (group "ported software", #439), and people install it on a
running Pi:

```sh
pkg install tuxtype            # SD-card image
pkg-persist install tuxtype    # network or direct boot (experiment pkg-persist)
```

The image needs the desktop (experiments `rpi-fb`, `orbital`, `desktop`),
relibc patch 0020 (on main with this PR) and, for German keyboards,
experiment [`keyboard-de`](../keyboard-de/README.md) (PR #581): the full
de map, AltGr and dead keys. `pkg` needs the network and experiment
`pkg-repo`.

**Building the packages:** this experiment adds the recipes and installs
Tux Typing into the build's image, so that the cookbook builds it and its
libraries. Then publish them:

```bash
scripts/build-image.sh --experiment timesync --experiment rpi-fb --experiment orbital \
    --experiment orbital-apps --experiment cosmic-apps-source --experiment desktop \
    --experiment tuxtype
scripts/publish-pkg-repo.sh tuxtype sdl1 sdl1-image sdl1-mixer sdl1-ttf freetype2 \
    gettext libiconv libjpeg libogg libpng libvorbis libxml2 xz zlib
```

- The package brings the launcher entry `72_tuxtype`, which starts
  `/usr/bin/tuxtype-de`. On the first start, that script writes
  `lang=deutsch` into `~/.config/tuxtype/settings.txt`, so Tux Typing
  starts in German. Tux Typing saves its settings, the language too, in
  that file. By hand: `tuxtype --theme deutsch`, or with `--window` for a
  window instead of full screen.
- t4kcommon is built without speech (there is no espeak on Redox, so
  `t4k_tts.c` is a stub) and without SDL_Pango, librsvg and SDL_net. It is
  static and linked into `tuxtype`. The other libraries are shared and
  come as dependencies of the `tuxtype` package.
- It needs relibc patch 0020 in the image's libc. Tux Typing reads its
  files line by line with `fscanf(f, "%[^\n]\n", ...)`. relibc's scanf
  skipped white space only for a space in the format, so it read the first
  line forever.
- **Size:** the `tuxtype` package is 21 MB (`/usr/share/tuxtype`: images
  12 MB, sounds 4.6 MB, themes 4.4 MB, fonts 2.8 MB), plus its libraries.
  On network boot it goes into RAM with the root filesystem. That is fine
  on the Pi 400 and the 3B+. On the Zero 2 W (512 MB) watch the free
  memory. The package build sets a 384 MiB filesystem; images that install
  the package later do not need that.

[`qemu-tuxtype-check.sh`](qemu-tuxtype-check.sh) types the German layout
test at the console. Then it starts the desktop and Tux Typing (from an
image built with this experiment) and walks to
lesson 1 with the keyboard, taking screenshots.

Limits:

- The menu help lines stay English. relibc's `setlocale(LC_ALL,
  "de_DE.UTF-8")` returns NULL, so gettext does not use the German
  translation (`/usr/share/locale/de/LC_MESSAGES/tuxtype.mo`). The theme's
  own words (menu entries, lessons) are German.
- No music: SDL_mixer here has no Ogg support for `Mix_Init`
  (`Failed to init required ogg and mod support`).
