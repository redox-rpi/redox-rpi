# Experiment: bcm2835-mboxd, firmware properties over the mailbox

New driver for the VideoCore mailbox (`brcm,bcm2835-mbox`). It asks the
Raspberry Pi firmware through the property interface and serves the answers
as text files, read fresh on every open:

| File | Content |
|---|---|
| `/scheme/mbox/temperature` | SoC temperature in millidegrees Celsius |
| `/scheme/mbox/throttled` | Throttling flags as hex (bit 0 undervoltage, 1 frequency capped, 2 throttled, 3 soft temperature limit; bits 16–19 the same since boot) |
| `/scheme/mbox/clocks` | One line per clock: name, current rate, maximum rate (Hz) |
| `/scheme/mbox/board` | `key value` lines: revision, model, memory, serial, firmware, memory split, maximum temperature |

```bash
scripts/build-image.sh --experiment mailbox-driver
```

Starts from the initfs, so other drivers (sdhost's clock divider) can ask
for the real core clock later.

**Result in QEMU** (`raspi3b`, build `v0.1.0-alpha-19-g9532b7f+mailbox-driver`,
2026-10-27): all four files answer. QEMU reports fixed values: revision
`0x00a02082` (Pi 3 Model B, 1 GiB), 25.0 °C, ARM 700 MHz, core 350 MHz, no
throttling. Not yet tested on a real Pi.
