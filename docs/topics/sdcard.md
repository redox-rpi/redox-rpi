# SD card

## State

- `bcm2835-sdhcid` (Arasan controller) is the default driver, patched for
  error recovery and a safe data clock.
- `bcm2835-sdhostd` (sdhost controller, the one Linux uses) is an
  experiment (`sdhost-driver`, `sdhost-boot`): 2.5 times faster writing and
  2.9 times faster reading than `bcm2835-sdhcid`.
- v0.3.0-rc1, an SD card image with sdhost as the only driver and the root
  filesystem on the card, passed on a real Pi 3B+ on 2026-10-28.

## Next

- [#6](https://github.com/redox-rpi/redox-rpi/issues/6) sdhost as the default driver: stress test, real core clock,
  experiments to base patches.
- [#16](https://github.com/redox-rpi/redox-rpi/issues/16) RedoxFS writes 36 times the data for small writes.

## Later

- sdhost for the card frees the Arasan controller for Wi-Fi.
