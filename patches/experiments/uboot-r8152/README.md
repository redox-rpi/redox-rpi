# Experiment: uboot-r8152, U-Boot's RTL8152 driver with a 16 KB receive buffer

Status: experimental, not tested on a Pi yet. Issue
[#778](https://github.com/redox-rpi/redox-rpi/issues/778).

An interim for network boot on the Pi Zero 2 W with the Ethernet/USB hub HAT
(RTL8152), until our open firmware netboots by itself
([rpi-open-firmware #87](https://github.com/redox-rpi/rpi-open-firmware/issues/87)).

## Why

U-Boot's `r8152` driver reads from the chip with a 2048-byte buffer
(`RTL8152_AGG_BUF_SZ`). The RTL8152 packs several Ethernet frames into one
USB transfer. When a burst does not fit, the read fails
(`Rx: failed to receive: -5`), so HTTP and full TFTP blocks fail.

Today `scripts/netboot.sh --board zero2w` therefore uses TFTP only, with
512-byte blocks: many minutes for the 42 MB image
([netboot.md](../../../docs/netboot.md)).

## What

[`u-boot/0001-r8152-16k-rx-buffer.patch`](u-boot/0001-r8152-16k-rx-buffer.patch)
raises the buffer to 16 KB, the size Linux uses. The driver already takes
several frames out of one buffer (`r8152_free_pkt` steps to the next one).

Build:

```bash
scripts/build-uboot.sh rpi --experiment uboot-r8152
```

`build/u-boot/experiments.txt` then names the experiment.

Our U-Boot uses the lwIP network stack. Its TFTP client knows `blksize` but
not `windowsize`, so the fast path is HTTP (lwIP `wget`, TCP), as on the 3B+.
TFTP with full blocks stays as the fallback.

## Test

On a Zero 2 W with the HAT, with a card that has this U-Boot:

```bash
scripts/netboot.sh --server <ip> --board zero2w --http
scripts/netboot.sh --server <ip> --board zero2w --tftp-only --tftp-blocksize 1468
```

Note, for each of the two runs:

- the load time and MB/s that U-Boot prints;
- the time from power-on to `redox login:`.
