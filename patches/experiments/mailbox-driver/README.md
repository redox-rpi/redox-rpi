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

**Result:** see below once tested.
