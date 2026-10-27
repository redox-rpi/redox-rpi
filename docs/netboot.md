# Network boot

Boot Redox on a real Pi straight from the build server: the SD card only
holds the Pi firmware, U-Boot and a boot script. On every power-on U-Boot
fetches the current Redox image from the build server. Build, power-cycle,
watch the serial console; no SD card swapping.

This is a setup for **developers**. To just try Redox on a Pi, flash the SD
card image from the
[latest release](https://github.com/redox-rpi/redox-rpi/releases) instead.
There are no prebuilt network boot files: the SD card stores your server's
address, so you build them on your own server.

## What you need

- A **build server**: x86_64 Linux that can build Redox, see
  [build-host.md](build-host.md). It also serves the image over TFTP and
  HTTP, so it must be on the same LAN as the Pi and should have a fixed
  address (a DHCP reservation in the router).
- A **Raspberry Pi 3 Model B+** with Ethernet to that LAN and a DHCP server
  (your router).
- A microSD card of at least 1 GB.
- Optional, but recommended: a USB serial adapter on the Pi's UART
  ([hardware.md](hardware.md)). The bootloader's menu still waits for
  **Enter** there (see Limits).

In the commands below, `<server-ip>` is your build server's LAN address,
`/srv/tftp` the directory it serves. Our own setup uses `192.168.178.120`.

The network boot image (`config/raspi3bp.toml`) also contains `devshd`, a
remote shell on port 2323 without encryption or extra password (it runs
`login`). Use it only on a LAN you trust; it is not in the release image.

```
Pi firmware → U-Boot ─┬─ DHCP (router) ───────────── IP address
                      ├─ TFTP (build server) ─────── netboot.scr
                      └─ HTTP (build server :8080) ─┬─ harddrive.img.gz → unzip in RAM → blkmap disk
                         (TFTP as fallback)         └─ bootloader-live.efi → copies RedoxFS into RAM → kernel
```

The flow is tested in QEMU's `virt` machine (virtio network, QEMU's built-in
TFTP server) up to `redox login:`. On the Pi, U-Boot reaches the network
through the USB Ethernet chip: LAN7800 on the Pi 3B+, RTL8152 on our Zero 2 W
HATs. Both drivers are in our U-Boot build.

## One-time setup on the build server

```sh
sudo apt-get install -y mtools tftpd-hpa
sudo install -d -o "$USER" /srv/tftp
```

`tftpd-hpa` serves `/srv/tftp` on UDP port 69. Give the server a fixed
address in the router (DHCP reservation), since the SD card stores it.
Set up the HTTP server as well (section "Transfer time" below); U-Boot
loads the image over HTTP on port 8080 and falls back to TFTP only if that
fails.

## After every build

```sh
scripts/build-image.sh
scripts/netboot.sh --server <server-ip> --tftp-dir /srv/tftp
```

Then power-cycle the Pi.

## The SD card

```sh
scripts/build-uboot.sh
scripts/make-sdcard.sh --tftp-dir /srv/tftp
```

Run this once, after the first `scripts/netboot.sh`: it takes `boot.scr`,
which holds `<server-ip>`, from the TFTP directory. Writes
`build/sdcard-netboot.img` (1 GiB, any card of that size or larger). Flash it
with Raspberry Pi Imager ("Use custom") or `dd`. If the server's address
changes, run `scripts/netboot.sh` with the new one and make a new card.

| Partition | Size | Content |
|---|---|---|
| 1, FAT32 | 63 MiB | Pi firmware (pinned release), [`config/config.txt`](../config/config.txt), our U-Boot, `boot.scr` |
| 2, RedoxFS | 960 MiB (`--data-size`) | Data that survives a reboot, mounted at **`/scheme/data`** |

`boot.scr` fetches `redox/netboot.scr` from the server, so changes to the boot
script also only need `scripts/netboot.sh`, not a new card.

## What the Pi loads

Three places decide it:

| Where | What it decides | Changed by |
|---|---|---|
| `boot.scr` on the SD card | server address and TFTP directory (`<dir>/netboot.scr`) | a new card (`make-sdcard.sh --dir`) |
| `<dir>/` on the server | the image (`harddrive.img.gz`), the bootloader, `netboot.scr` | `scripts/netboot.sh --dir <dir>` |
| U-Boot variables | `redox_dir`, `redox_server`, `redox_http`, the RAM addresses | `setenv` on the serial console before `netboot.scr` runs |

So the normal way to change what a Pi boots is to run `scripts/netboot.sh`
with another image: every card that names that directory boots it on the
next power cycle. Boards that need different images get different
directories: the Pi 3B+ uses `redox/`, the Zero 2 W `redox-zero2w/`. The
manifest of the image in a directory is `<dir>/manifest.toml`
([versioning.md](versioning.md)).

To boot another directory once, without a new card, stop U-Boot's countdown
on the serial console and run:

```
setenv redox_dir backup/; usb start; dhcp; setenv serverip <server-ip>
tftpboot ${scriptaddr} redox/netboot.scr; source ${scriptaddr}
```

## Pi Zero 2 W

The Zero has 512 MB RAM and no Ethernet of its own: U-Boot reaches the
network through the RTL8152 on the Ethernet/USB hub HAT. It gets its own
image, 128 MiB instead of 256, and its own directory:

```sh
scripts/build-image.sh --zero2w
scripts/netboot.sh --server <server-ip> --board zero2w --tftp-dir /srv/tftp
scripts/make-sdcard.sh --tftp-dir /srv/tftp --dir redox-zero2w \
    --boot-size 32 --data-size 0 --out build/sdcard-netboot-zero2w.img
```

The card is 32 MiB, small enough for a 64 MB SD card, with no data
partition. The same card boots a Pi 3 as well, with the Zero's image.

Tested in QEMU's `virt` machine with `-m 512` and the Zero's RAM addresses
(`redox_img_addr`/`redox_gz_addr` 128 and 256 MiB above the start of RAM):
it boots to `redox login:`.

On a real Zero 2 W, U-Boot gets DHCP and loads `netboot.scr` over TFTP
through the HAT's RTL8152, but the HTTP transfer of the image fails with
`Rx: failed to receive: -5` from U-Boot's r8152 driver. So `--board zero2w`
loads everything over TFTP (`--tftp-only`, `redox_http none`): slower, about
a minute for the image.

TFTP with U-Boot's default 1468-byte blocks failed the same way on the
Zero, after a few dozen KiB of the image. The driver receives into a
2048-byte buffer (`RTL8152_AGG_BUF_SZ`), and the adapter packs frames that
arrive close together into one USB transfer; two full-size frames do not
fit. `--board zero2w` therefore also sets `tftpblocksize 512`
(`--tftp-blocksize`). With it, a real Zero 2 W loaded the whole image and
booted Redox to the login prompt (2026-10-27). It is slow: many minutes for
the 42 MB image, at times only a few KB/s, not timed exactly. A larger
receive buffer in U-Boot's driver would allow full blocks and HTTP again.

The Redox system itself runs from RAM and starts fresh on every boot; keep
anything worth keeping in `/scheme/data`. It is a scheme path, not `/data`:
Redox does not follow symlinks into another scheme. Mounting a RedoxFS
partition by path and finding the data again after a reboot is tested in
QEMU; the netboot card's own layout only on real hardware.

## Testing the flow in QEMU

```sh
scripts/build-uboot.sh qemu
scripts/netboot.sh --server 10.0.2.2 --board qemu --tftp-dir build/tftp-qemu
qemu-system-aarch64 -M virt -cpu cortex-a53 -smp 4 -m 2048 \
    -bios build/u-boot-qemu/u-boot.bin \
    -netdev user,id=n0,tftp=build/tftp-qemu -device virtio-net-device,netdev=n0 \
    -serial mon:stdio -display none
```

Stop U-Boot's countdown with a key, then:

```
setenv autoload no; dhcp; setenv serverip 10.0.2.2
tftpboot ${scriptaddr} redox/netboot.scr; source ${scriptaddr}
```

## Transfer time

TFTP sends one small block and waits for the acknowledgement, so every
millisecond of network latency counts (the build server is on Wi-Fi). Two
measures make it fast:

- The 256 MiB image is mostly empty space: `scripts/netboot.sh` gzips it to
  about 42 MB and U-Boot unpacks it in RAM.
- The image and the bootloader come over **HTTP** with U-Boot's `wget`
  (lwIP network stack), TFTP is only the fallback. TCP does not wait for every
  block.

| Transfer of the image | Time |
|---|---|
| TFTP, uncompressed (Mac over Wi-Fi) | 8:47 min |
| TFTP, gzip (Mac over Wi-Fi) | 1:19 min |
| HTTP, gzip (Mac over Wi-Fi) | 3.6 s |
| HTTP, gzip (QEMU on the server) | 0.55 s |

The HTTP server is a systemd user service on the build server:

```ini
# ~/.config/systemd/user/redox-http.service
[Unit]
Description=HTTP server for Redox network boot (/srv/tftp)

[Service]
ExecStart=/usr/bin/python3 -m http.server 8080 --directory /srv/tftp
Restart=on-failure

[Install]
WantedBy=default.target
```

```sh
systemctl --user enable --now redox-http.service
sudo loginctl enable-linger "$USER"   # keep it running without a login
```

## Limits

- The bootloader's resolution menu still waits for **Enter** on the serial
  console.
- RAM: the unpacked image (256 MiB) sits at 256 MiB, the compressed one
  after it, and the bootloader copies the 222 MiB RedoxFS once more. That
  fits the Pi 3B+ (1 GB), but not the Zero 2 W (512 MB), which gets the
  128 MiB image (`--board zero2w`, see above). Transferring only the
  RedoxFS partition would save more.
- On the Zero 2 W, Redox has no network driver for the HAT's RTL8152 yet
  (experiment `rtl8152`, branch `rtl8152`): without it, only the serial
  console reaches the shell.
