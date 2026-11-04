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
- The link-up lines name the speed: `dwc2d: link up on the LAN7800 at
  address N, 1000 Mbit/s`, at boot and after a change. (`dwc2d: LAN78xx
  link up after N ms, …` only appears for an adapter plugged in later.)

**Pi test 228:** 100 Mbit/s saves about **0.35 W** at idle: gigabit 3.23
and 3.24 W, 100 Mbit/s 2.89 and 2.88 W (alternating, 36 readings over
6 minutes each). Every change came back within 4 s, ping 3 of 3, pi-test
0 of 300 failed starts.

## Boot setting

`/etc/eth-speed` with `100` (or `1000`): at boot `16_eth-speed.service`
copies it to `/tmp/dwc2d-link`, and dwc2d renegotiates a few seconds after
boot (DHCP tries again meanwhile). Without the file nothing changes.
Experiment [`eth-speed-test`](../eth-speed-test/README.md) writes the file
with `100` for a test image. Pi test 231: the link came up at gigabit and
went to 100 Mbit/s when the rootfs services started; idle 2.88 W.

**From the first negotiation (base `0092`):** `RPI_ETH_SPEED=100` (or
`1000`) in the kernel environment or on the firmware's command line
(`cmdline.txt`, which the firmware puts into the devicetree's `/chosen
bootargs`) makes dwc2d advertise only up to that when it first brings up
the LAN7800, so the link never starts at gigabit. dwc2d logs
`dwc2d: RPI_ETH_SPEED=100 from the command line` (or `environment`) at
start and `dwc2d: LAN78xx advertising up to 100 Mbit/s (RPI_ETH_SPEED)`.
On a direct-boot card the stub puts `cmdline.txt` words into the
environment; whether `/chosen bootargs` survives U-Boot and the Redox
bootloader on network boot is for the Pi test to show.

## Measuring

[`shelly-sample.sh`](shelly-sample.sh) reads the Shelly plug's power and
averages it (it only reads, never switches). The plug resolves 0.1 W, so
each setting gets several minutes of samples, and the settings alternate
to cancel drift.

## QEMU

[`qemu-check.sh`](qemu-check.sh): QEMU's USB network card is CDC Ethernet,
not a LAN7800, so it checks that dwc2d reads and refuses the request and
keeps running. The speed change itself needs the Pi.
