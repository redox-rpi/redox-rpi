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

Switch on the running system:

```bash
inputd -K de
```

A setting that picks the layout at boot comes next.

## Test in QEMU

```bash
patches/experiments/keyboard-de/test-keymap.sh [image]
```

It runs `inputd -K de` on the serial console, logs in on the console with
QEMU's USB keyboard, types `echo 'zyß@{[]}\~|#öäü€"§' > /home/root/de.txt`
as the keys of a German keyboard, and checks the file. It needs no
network port.

## Results

See [test-reports.md](../../../docs/test-reports.md).
