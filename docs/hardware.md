# Hardware notes

## Boards

| | Pi 3 Model B | Pi 3 Model B+ | Pi Zero 2 W |
|---|---|---|---|
| SoC | BCM2837 | BCM2837B0 | BCM2710A1 (RP3A0 SiP) |
| CPU | 4× Cortex-A53 1.2 GHz | 4× Cortex-A53 1.4 GHz | 4× Cortex-A53 1.0 GHz |
| RAM | 1 GB | 1 GB | 512 MB |
| Ethernet | LAN9514 (USB hub + 100 Mbit) | LAN7515 (USB hub + Gigabit over USB 2), see below | none (Ethernet/USB hub HAT) |
| Firmware DTB | `bcm2710-rpi-3-b.dtb` | `bcm2710-rpi-3-b-plus.dtb` | `bcm2710-rpi-zero-2-w.dtb` |
| Mainline DTB (U-Boot/Linux) | `bcm2837-rpi-3-b.dtb` | `bcm2837-rpi-3-b-plus.dtb` | `bcm2837-rpi-zero-2-w.dtb` |

All three share the same peripherals at the same addresses (peripheral base
`0x3F000000`), so one kernel and driver set covers them.

### Our Pi 3 Model B+ (rev 1.3, revision code `a020d3`)

USB tree under Raspberry Pi OS (Linux 6.18, `lsusb -t`):

```
DWC2 root hub (dwc_otg)
└─ 0424:2514 USB 2.0 hub (4 ports)
   └─ 0424:2514 USB 2.0 hub (3 ports)
      └─ 0424:7800 Ethernet, Linux driver lan78xx
```

So Ethernet on the Pi 3B+ needs: DWC2 host driver → two levels of USB 2.0
hubs → a LAN7800 driver. The SD card runs on `sdhost`; the Arasan
controller (`mmc-bcm2835 3f300000.mmcnr`) drives Wi-Fi.

## Serial console

Everything early happens on the serial console, so this is the most
important piece of hardware.

- Use a **3.3 V** USB-UART adapter (CP2102, FT232, CH340). 5 V adapters
  damage the Pi.
- Wiring (same pins on all three boards):

  | Pi pin | Signal | Adapter |
  |---|---|---|
  | 6 | GND | GND |
  | 8 | GPIO14 / TXD | RXD |
  | 10 | GPIO15 / RXD | TXD |

  Do not connect the adapter's VCC; power the Pi through its own USB port.
- 115200 baud, 8N1. On macOS: `screen /dev/tty.usbserial-* 115200`
  (exit with Ctrl-A K).
- The Zero 2 W needs a soldered pin header.

Redox only supports the PL011 UART. On these boards the PL011 drives
Bluetooth by default, so `config.txt` must contain `dtoverlay=disable-bt`
to put the PL011 on GPIO14/15.

## Boot partition (`config.txt`)

The config used by Redox's Pi 3B+ port
([redox_firmware](https://gitlab.redox-os.org/Ivan/redox_firmware)):

```ini
arm_64bit=1
dtoverlay=disable-bt
enable_uart=1
kernel=u-boot.bin
```

The EFI partition additionally holds the Pi firmware files (`bootcode.bin`,
`start.elf`, `fixup.dat`, DTBs, overlays) from
[raspberrypi/firmware](https://github.com/raspberrypi/firmware) `boot/`.
The Pi boot ROM only reads MBR, so the GPT image needs a hybrid MBR (see the
[Redox book](https://doc.redox-os.org/book/aarch64.html)).

## Ethernet/USB hub HAT (Zero 2 W)

These HATs usually connect through pogo pins to the Zero's USB test pads and
contain a USB hub chip plus a USB Ethernet chip. Before writing a driver,
identify both chips under Raspberry Pi OS:

```sh
lsusb
lsusb -t
dmesg | grep -i -E "usb|eth|r815|asix|smsc"
```

Note vendor:product IDs and the Linux driver name in this file.

| HAT | Hub chip | Ethernet chip | Linux driver |
|---|---|---|---|
| (to be filled in) | | | |

## Debugging without serial output

If nothing appears on the serial console:

1. Check with Raspberry Pi OS that wiring and adapter work.
2. Check U-Boot alone (prompt on serial).
3. JTAG is available on GPIO22–27 (ALT4, `enable_jtag_gpio=1`), usable with
   OpenOCD and a cheap adapter. Only needed if the kernel dies before the
   UART is initialized.
