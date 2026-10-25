# Network boot

Boot Redox on a real Pi straight from the build server: the SD card only
holds the Pi firmware, U-Boot and a boot script. On every power-on U-Boot
fetches the current Redox image over TFTP. Build, power-cycle, watch the
serial console; no SD card swapping.

```
Pi firmware → U-Boot ── DHCP (router) ──────────────── IP address
                     └─ TFTP (build server) ── netboot.scr
                                              ├─ harddrive.img → RAM → blkmap disk
                                              └─ bootloader-live.efi → copies RedoxFS into RAM → kernel
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

## After every build

```sh
scripts/build-image.sh
scripts/netboot.sh --server 192.168.178.120 --tftp-dir /srv/tftp
```

Then power-cycle the Pi.

## The SD card

```sh
scripts/build-uboot.sh
scripts/make-sdcard.sh --tftp-dir /srv/tftp
```

Writes `build/sdcard-netboot.img` (1 GiB, any card of that size or larger).
Flash it with Raspberry Pi Imager ("Use custom") or `dd`.

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

## Limits

- The bootloader's resolution menu still waits for **Enter** on the serial
  console.
- RAM: the image (256 MiB, half of it the EFI partition) is loaded at
  256 MiB and the bootloader copies the 126 MiB RedoxFS once more. That fits
  the Pi 3B+ (1 GB), but not the Zero 2 W (512 MB). The Zero needs a smaller
  image or a transfer of the RedoxFS partition only.
