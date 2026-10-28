# Network

## State

- LAN7800 (Pi 3B+ onboard Ethernet) in `dwc2d`: DHCP, TCP from the LAN.
- RTL8152/RTL8153 USB Ethernet (base patch `0020`, since v0.2.0): the
  Zero 2 W gets its time over the HAT's RTL8152.
- `devshd`: remote shell on port 2323, unencrypted, LAN only, never in
  release images.

## Next

- Confirm the Zero 2 W's IP address and TCP (`timesync -n` worked).

## Later

- [#26](https://github.com/redox-rpi/redox-rpi/issues/26) Wi-Fi on the Pi 3B+ and Zero 2 W (milestone v0.6.0); it would be the
  first Wi-Fi in Redox.
- [#27](https://github.com/redox-rpi/redox-rpi/issues/27) IPv6 in `netstack` (milestone v0.6.0); Matter runs over IPv6.
- SSH and Bluetooth ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
