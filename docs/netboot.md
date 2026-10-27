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

The Redox system itself runs from RAM and starts fresh on every boot; keep
anything worth keeping in `/scheme/data`. It is a scheme path, not `/data`:
Redox does not follow symlinks into another scheme. Mounting a RedoxFS
partition by path and finding the data again after a reboot is tested in
QEMU; the netboot card's own layout only on real hardware.

## Without SD card (firmware network boot, untested)

The Pi 3B+ boot ROM can network-boot by itself when there is no SD card in
it: it fetches the Pi firmware over TFTP, the firmware loads U-Boot, and
U-Boot continues as above. Not yet tried on our Pi.

```
Boot ROM ─ DHCP (router + dnsmasq proxy) ─ TFTP bootcode.bin
  → bootcode.bin ─ TFTP start.elf, config.txt, devicetree, u-boot.bin
  → U-Boot (u-boot-net) ─ TFTP redox/netboot.scr → as above
```

This only adds files to the TFTP root and does not change `redox/`: a Pi
with the netboot SD card in it keeps booting from the card, so it does not
get in the way of other tests.

```sh
scripts/build-uboot.sh rpi-net <server-ip>
scripts/netboot-firmware.sh --tftp-dir /srv/tftp
```

`rpi-net` is U-Boot with the steps of `boot.scr` built in as its boot
command, since without a card there is no `boot.scr`.

The boot ROM waits for a DHCP answer with "Raspberry Pi Boot" in it, which
a home router does not send. dnsmasq adds it as a **proxy DHCP server** next
to the router: it hands out no addresses and answers only PXE requests from
Pi network adapters ([`config/dnsmasq-pxe-proxy.conf`](../config/dnsmasq-pxe-proxy.conf);
change `dhcp-range` to your network). U-Boot and Redox send no PXE requests
and are not affected. One-time setup on the build server:

```sh
sudo apt-get install -y dnsmasq-base
sudo install -m 644 config/dnsmasq-pxe-proxy.conf /etc/redox-pxe-proxy.conf
sudo install -m 644 config/redox-pxe-proxy.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now redox-pxe-proxy.service
```

To test: take the SD card out, power on, and watch
`journalctl -fu redox-pxe-proxy -u tftpd-hpa`. The TFTP log shows which
files the Pi asks for. If it asks for `<serial>/start.elf`, the files can go
there instead with `--serial <serial>`, so each Pi gets its own.

Open:

- The 3B+ ships with network boot enabled in its OTP; whether ours boots
  this way has to be tried.
- The boot ROM's network code is known to be picky. With a card that holds
  only `bootcode.bin` the newer code from that file does the network boot.
- Later (T15) `u-boot.bin` can be replaced by the Redox kernel and initfs.

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
- RAM: the unpacked image (256 MiB, half of it the EFI partition) sits at
  256 MiB, the compressed one after it, and the bootloader copies the
  126 MiB RedoxFS once more. That fits the Pi 3B+ (1 GB), but not the
  Zero 2 W (512 MB). The Zero needs a smaller image (EFI partition) or a
  transfer of the RedoxFS partition only.
