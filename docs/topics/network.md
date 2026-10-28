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

- SSH and Wi-Fi ([wish list](../../ROADMAP.md#wish-list-help-wanted)).
