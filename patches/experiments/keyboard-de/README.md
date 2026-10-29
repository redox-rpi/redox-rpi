# Experiment: keyboard-de, German keyboard layout (#36)

The USB keyboard typed with the US layout only. `dwc2d` sends the key's
position (a scancode); `inputd` turns it into a character with the active
layout. `inputd` had maps for `us`, `gb`, `dvorak`, `azerty`, `bepo` and
`it`, but no German map and no AltGr ("TODO: AltGr").

```bash
scripts/build-image.sh --experiment timesync --experiment rtl8152 --experiment keyboard-de
```

Base patch
[`0025`](base/0025-inputd-German-keyboard-layout-de-with-AltGr.patch)
(only `drivers/inputd`, no `dwc2d` change):

- a `de` map (T1): z and y swapped, ß ü ö ä, the # key left of Enter, the
  < > key right of the left Shift (scancode `0x56`, which `dwc2d` already
  sends);
- AltGr: @ € { [ ] } \ ~ | ² ³ µ. `inputd` keeps the right Alt key's state;
  layouts without an AltGr table ignore it as before;
- the dead keys ^ ´ ` type their character at once (no é from ´ e yet).

An image built with this experiment types German from boot: its
[`config.toml`](config.toml) adds `/etc/init.d/05_keymap.service`, which
runs `inputd -K de` once the root file system is up (inputd itself starts
in the initfs with US). On an SD card image, edit the unit for another
layout (`inputd --keymaps` lists them) or remove it. Switch on the running
system:

```bash
inputd -K us
```

## Test in QEMU

```bash
patches/experiments/keyboard-de/test-keymap.sh [image]
```

It boots the image (German from boot), logs in on the console with
QEMU's USB keyboard, types `cat > /home/root/de.txt` and the line
`zyß@{[]}\~|#öäü€"§'<>` (then Ctrl-D) as the keys of a German keyboard,
and checks the file. The characters go to `cat`, not to ion, which
rejects `[` even in quotes. It needs no network port.

## Results

- QEMU `raspi3b`, `v0.3.0-dev-141-g3fd1bba+x9ea9` (with `timesync`,
  `rtl8152`): `test-keymap.sh` passes, every character as on a German
  keyboard, AltGr included.
- QEMU `raspi3b`, `v0.3.0-dev-143-ge940bf6+x9ea9` (German from boot, no
  `inputd -K` by hand): `test-keymap.sh` passes the same way.

Details in [test-reports.md](../../../docs/test-reports.md).
