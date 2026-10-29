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
- dead keys (base patch
  [`0026`](base/0026-inputd-dead-keys-for-the-German-layout-keymap-from-a.patch)):
  ´, ` and ^ wait for the next key. With a vowel they make á é í ó ú ý,
  à è ì ò ù, â ê î ô û (and capitals); with space they type themselves;
  with any other key they type themselves and then that key.

An image built with this experiment types German from boot: its
[`config.toml`](config.toml) adds `/etc/keymap` with `de` and a unit that
runs `inputd --keymap-file /etc/keymap` once the root file system is up
(inputd itself starts in the initfs with US). To change the layout, write
another name to `/etc/keymap` (`inputd --keymaps` lists them); on an SD
card image it stays for the next boot. Without the file the layout stays
US. Switch on the running system:

```bash
inputd -K us
```

## Test in QEMU

```bash
patches/experiments/keyboard-de/test-keymap.sh [image]
```

It boots the image (German from boot), logs in on the console with
QEMU's USB keyboard, types `cat > /home/root/de.txt` and the line
`zyß@{[]}\~|#öäü€"§'<>éàô´^x` (then Ctrl-D; the last six from the dead
keys) as the keys of a German keyboard,
and checks the file. The characters go to `cat`, not to ion, which
rejects `[` even in quotes. It needs no network port.

## Results

- QEMU `raspi3b`, `v0.3.0-dev-141-g3fd1bba+x9ea9` (with `timesync`,
  `rtl8152`): `test-keymap.sh` passes, every character as on a German
  keyboard, AltGr included.
- QEMU `raspi3b`, `v0.3.0-dev-143-ge940bf6+x9ea9` (German from boot, no
  `inputd -K` by hand): `test-keymap.sh` passes the same way.
- QEMU `raspi3b`, `v0.3.0-dev-239-g43c6882+x9ea9` (dead keys, layout from
  `/etc/keymap`): `test-keymap.sh` passes with `éàô´^x` at the end of the
  line. Pi test with a person: #64 in the test plan.

Details in [test-reports.md](../../../docs/test-reports.md).
