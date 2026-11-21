# Experiment: keymap-de, the German keyboard layout

For [#529](https://github.com/redox-rpi/redox-rpi/issues/529) (Tux Typing
with German lessons) and German keyboards such as the Pi 400's German model.
At boot, `inputd -K de` selects the German layout for the text console and
the desktop: QWERTZ, ß, ü, ö, ä, and the ISO keys `# '` and `< >`. Base
patch 0102 adds the layout to inputd and fixes usbhidd's mapping of the two
ISO keys.

By hand, without this experiment: `inputd -K de` as root, and `inputd -K us`
goes back. `inputd --keymaps` lists the layouts.

## Limits

- No AltGr: `@`, `€`, `{`, `[`, `]`, `}`, `\`, `~` and `|` cannot be typed
  with this layout. inputd has no AltGr level for any layout yet.
- The dead keys `´` and `^` type their own character at once.
