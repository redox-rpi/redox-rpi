# Experiment: transfer statistics of the sdhost driver

Sets `SDHOSTD_STATS` for `bcm2835-sdhostd` at boot. While there is disk
activity, the driver prints every 5 seconds how many read and write
requests it got, how many blocks they had, how long they took, and how much
of the write time the card spent programming (busy):

```text
bcm2835-sdhostd: 5 s: read 3 requests 24 blocks 4 ms; write 120 requests 960 blocks 2900 ms, 2400 ms of it busy
```

Needs [`sdhost-driver`](../sdhost-driver/README.md) and
[`sdhost-boot`](../sdhost-boot/README.md) before it:

```bash
scripts/build-image.sh --experiment ldso-diagnostics --experiment sdhost-driver --experiment sdhost-boot --experiment sdhost-stats
```

## Results

None yet.
