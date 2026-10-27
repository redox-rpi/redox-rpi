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

2026-10-27, real Pi 3B+, `v0.1.0-alpha-61-g90ef0ee+ldso-diagnostics+sdhost-driver+sdhost-boot+sdhost-stats`: works. While `dd` wrote 1 MiB in 4 KiB
calls with `conv=fsync`:

```text
bcm2835-sdhostd: 31 s: read 0 requests 0 blocks 0 ms; write 524 requests 15968 blocks 959 ms, 278 ms of it busy
bcm2835-sdhostd: 5 s: read 288 requests 3952 blocks 197 ms; write 1763 requests 49072 blocks 4034 ms, 1414 ms of it busy
bcm2835-sdhostd: 36 s: read 127 requests 1016 blocks 63 ms; write 307 requests 9152 blocks 571 ms, 162 ms of it busy
```

About 74,000 blocks (37 MB) for 1 MiB of data. Details in the
[sdhost-driver README](../sdhost-driver/README.md#results).
