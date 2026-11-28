# Experiment: keyboard-de, German keyboard layout (#36)

**Merged:** the patches are base patches in every image now:
[`0136`](../../base/0136-inputd-German-keyboard-layout-de-with-AltGr.patch)
(de map, AltGr),
[`0137`](../../base/0137-inputd-dead-keys-for-the-German-layout-keymap-from-a.patch)
(dead keys, `--keymap-file`),
[`0138`](../../base/0138-usbhidd-ISO-keys-and-as-dwc2d-sends-them-redox-rpi-3.patch)
(usbhidd ISO keys) and
[`0139`](../../base/0139-inputd-keyboard-layout-at-boot-from-KEYMAP-in-cmdlin.patch)
(layout at boot). `--experiment keyboard-de` is accepted and does nothing.

The USB keyboard typed with the US layout only. `dwc2d` sends the key's
position (a scancode); `inputd` turns it into a character with the active
layout. `inputd` had maps for `us`, `gb`, `dvorak`, `azerty`, `bepo` and
`it`, but no German map and no AltGr ("TODO: AltGr").

## What the patches do

- a `de` map (T1): z and y swapped, ß ü ö ä, the # key left of Enter, the
  < > key right of the left Shift (scancode `0x56`, which `dwc2d` already
  sends);
- AltGr: @ € { [ ] } \ ~ | ² ³ µ. `inputd` keeps the right Alt key's state;
  layouts without an AltGr table ignore it as before;
- dead keys: ´, ` and ^ wait for the next key. With a vowel they make
  á é í ó ú ý, à è ì ò ù, â ê î ô û (and capitals); with space they type
  themselves; with any other key they type themselves and then that key;
- `usbhidd`, the keyboard driver behind xHCI (Pi 4 and Pi 400), sends the
  key left of Enter as `K_BACKSLASH` and the key right of the left Shift as
  `0x56`, as `dwc2d` does, so `#` and `< >` work on the Pi 400 keyboard;
- the layout at boot: the unit `05_keymap` runs `inputd --keymap-boot`
  before the consoles start. It takes `KEYMAP=<name>` from the kernel
  command line, else the name in `/etc/keymap`, else the layout stays
  `us`. On a Raspberry Pi the command line is `cmdline.txt` on the card's
  FAT partition (the firmware puts it into the devicetree's `/chosen
  bootargs`), so the layout of a card is one word in one file, with no
  login. Network boot reads `cmdline.txt` from the card too.
- `inputd --keymap` prints the active layout; orblogin (experiment
  `orbital`, orbutils `0002`) shows it in its layout menu.

Choose the layout on the card: put this line into `cmdline.txt`
(`inputd --keymaps` lists the names):

```text
KEYMAP=de
```

Switch on the running system:

```bash
inputd -K us
```

## Test in QEMU

```bash
scripts/test-keymap.sh [image]
```

It boots the image with `KEYMAP=de` on the command line (QEMU `-append`,
`scripts/qemu-raspi3b.sh --append`), logs in on the console with
QEMU's USB keyboard, types `cat > /home/root/de.txt` and the line
`zyß@{[]}\~|#öäü€"§'<>éàô´^x` (then Ctrl-D; the last six from the dead
keys) as the keys of a German keyboard, and checks the file and that
`inputd --keymap` prints `de`. The characters go to `cat`, not to ion,
which rejects `[` even in quotes. It needs no network port.

## Results

- QEMU `raspi3b`, `v0.3.0-dev-141-g3fd1bba+x9ea9` (with `timesync`,
  `rtl8152`): `test-keymap.sh` passes, every character as on a German
  keyboard, AltGr included.
- QEMU `raspi3b`, `v0.3.0-dev-143-ge940bf6+x9ea9` (German from boot, no
  `inputd -K` by hand): `test-keymap.sh` passes the same way.
- QEMU `raspi3b`, `v0.3.0-dev-239-g43c6882+x9ea9` (dead keys, layout from
  `/etc/keymap`): `test-keymap.sh` passes with `éàô´^x` at the end of the
  line.
- QEMU `raspi3b`, `v0.3.0-dev-765-gc992087+x9ea9` (branch `t36-keymap`,
  today's main): `test-keymap.sh` passes the same way. Pi test with a
  person: #150 in the test plan (replaces #64).
- QEMU `raspi3b`, `v0.6.0-dev-121-g45a6707c+x5890` (moved from draft PR
  #132 onto main of 2026-11-20): `test-keymap.sh` passes. The script now
  waits 30 s instead of 10 s before typing: the framebuffer getty comes up
  after the serial login, and keys sent before it were lost. `0043`
  (usbhidd) has no QEMU test: QEMU `raspi4b` has no xHCI.

Details in [test-reports.md](../../../docs/test-reports.md).
