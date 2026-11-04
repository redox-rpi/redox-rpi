# Experiment: eth-speed

Issue [#126](https://github.com/redox-rpi/redox-rpi/issues/126), power-saving
mode, step 1: does the Pi 3B+ draw less at idle with its Ethernet at
100 Mbit/s instead of gigabit? On Linux a gigabit link on the LAN7515 costs
about 0.1-0.2 W more. Pi test 147 (experiment `power-save`, branch
`power-save`) found nothing else that saves 0.1 W or more.

```bash
scripts/build-image.sh --experiment eth-speed
```

## Base patch `0090`: `dwc2d` link speed at run time

- `echo 100 > /tmp/dwc2d-link`: the LAN7800 in use stops advertising
  1000BASE-T (PHY register 9, CTRL1000) and negotiates again. The link is
  down for a few seconds; the network comes back on its own.
- `echo 1000 > /tmp/dwc2d-link`: gigabit again.
- dwc2d reads the file once a second (with its USB rescan) and deletes it.
  Anything else, or no LAN7800 in use, is refused with a log line.
- The link-up lines name the speed: `dwc2d: LAN78xx link up after N ms,
  1000 Mbit/s` at boot, `dwc2d: link up on the LAN7800 at address N,
  100 Mbit/s` after a change.

If 100 Mbit/s saves enough, a later step makes it a boot setting.

## Measuring

[`shelly-sample.sh`](shelly-sample.sh) reads the Shelly plug's power and
averages it (it only reads, never switches). The plug resolves 0.1 W, so
each setting gets several minutes of samples, and the settings alternate
to cancel drift.

## QEMU

[`qemu-check.sh`](qemu-check.sh): QEMU's USB network card is CDC Ethernet,
not a LAN7800, so it checks that dwc2d reads and refuses the request and
keeps running. The speed change itself needs the Pi.
