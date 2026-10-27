# Experiment: RTL8152 Ethernet

Driver in `dwc2d` for the Realtek RTL8152 (`0bda:8152`), the USB 10/100
Ethernet chip on the Ethernet/USB hub HAT for the Pi Zero 2 W (behind its
FE1.1s hub) and on many cheap USB Ethernet adapters. Roadmap step 5.5.

```bash
scripts/build-image.sh --experiment rtl8152
```

Written with FreeBSD's `if_ure` (BSD-2-Clause) as reference for the register
map and the initialization order; no code from Linux' `r8152.c`. Like the
LAN7800 driver it polls and has no interrupt endpoint.

- Chip versions 4C00 and 4C10 (RTL8152 and RTL8152B). Any other version
  (RTL8153 and later share the product family but need another init) is
  refused with `dwc2d: RTL8152: Timeout("unsupported RTL815x chip version")`.
- MAC address from the chip's efuse (`PLA_IDR`, else `PLA_BACKUP`), else
  the locally administered `02:52:65:64:6f:78`.
- Autonegotiation 10/100; `dwc2d` waits up to 5 s for the link before the
  network stack starts. No link changes are reported afterwards.
- Receives all multicast (IPv6 neighbor discovery), no promiscuous mode, no
  checksum offload, no VLAN tags.
- The chip must hang off a high-speed hub; behind a transaction translator
  (full speed) it is skipped, because `dwc2d` has no split bulk transfers.
- **A plugged-in RTL8152 wins over the onboard LAN7800.** That makes a test
  on a Pi 3B+ with a USB adapter possible before the Zero 2 W image exists.
  The network scheme is then `network.usb_rtl8152`; the LAN7800 stays
  initialized but unused.

Patch `0017` conflicts with the other experiments that patch `dwc2d` or add
a base patch `0017` (`dwc2-interrupts`, `sdhost-driver`, `rpi-firmware`).

## Result

**QEMU** cannot emulate an RTL8152, so the test there only shows that
nothing else breaks. Build `v0.1.0-alpha-27-gee91c40+ldso-diagnostics+rtl8152`
(2026-10-27, `harddrive.img` `f6374a31…`), `scripts/qemu-raspi3b.sh --usb-net`:
boots to the login, enumerates hub, keyboard and CDC Ethernet as before,
`network.usb_ecm` is there and `ping -c 3 10.0.2.2` gets 3 of 3 answers.
The new code compiles without warnings.

**Real hardware:** not yet tested, see below.

## Test on hardware

### A. Pi 3B+ with a USB RTL8152 adapter

Needs a person at the Pi to plug in the adapter. Any adapter that reports
`0bda:8152` under Linux (`lsusb`) works. The HAT for the Zero cannot be used
here: it connects to the Zero's USB test pads, not to a USB port.

1. Plug the adapter into one of the Pi's USB ports and connect it to the
   same LAN. Keep the onboard Ethernet cable: U-Boot still netboots over the
   LAN7800.
2. Build and stage, with the stable diagnostics image as base:
   ```bash
   scripts/build-image.sh --experiment ldso-diagnostics --experiment rtl8152
   scripts/netboot.sh --server 192.168.178.120 --tftp-dir /srv/tftp --image upstream/redox/build/aarch64/minimal/harddrive.img
   ```
3. Reboot the Pi and read the serial console. Expected:
   ```text
   dwc2d: RTL8152 at address <n>
   dwc2d: RTL8152 chip version 4c00          (or 4c10)
   dwc2d: RTL8152 link up after <ms> ms, 100 Mbit/s full duplex
   dwc2d: network adapter <MAC of the adapter>
   ```
4. The Pi gets a DHCP address for the adapter's MAC (router's lease list).
   Check from the build host:
   ```bash
   ping -c 20 <address>
   scripts/pi-test.py <address>
   ```
   `pi-test.py` goes over `devshd` (TCP 2323), so its standard checks also
   test larger TCP transfers in both directions.
5. Pull the adapter's cable for 10 s and plug it back in: note whether
   `ping` comes back (the driver does not handle link changes yet).
6. Write the build ID, the chip version and the results into
   [test-reports.md](../../../docs/test-reports.md) and the "Result" section
   above.

If it fails, the serial log says where: `request [...] failed` names the
register access (wValue = register, wIndex = space and byte enables),
`Timeout("link list")` or `Timeout("reset")` a step of the chip init.

### B. Pi Zero 2 W with the Ethernet/USB hub HAT

The real target. Needs a Zero 2 W image first (roadmap phase 3: U-Boot with
the firmware's DT, 512 MB RAM, board variant). The checks are the same as in
A, steps 3 to 6. Before that, under Linux on the Zero, `lsusb -t` should show
the RTL8152 at 480M behind the FE1.1s hub; at 12M it would be skipped (see
above).
