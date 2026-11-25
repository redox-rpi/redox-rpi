# Network configuration file

`/etc/net/config.toml` says how the network starts
([#622](https://github.com/redox-rpi/redox-rpi/issues/622)). It has one
section per interface.

```toml
[interface.eth0]
ipv4 = "dhcp"
ipv6 = "off"
```

## Keys

Per `[interface.<name>]` section:

| Key | Values | Now |
|---|---|---|
| `boards` | list of board names, e.g. `["raspi3bp"]` | **do not use yet** ([#653](https://github.com/redox-rpi/redox-rpi/issues/653)): the board name is fixed at build time and is `raspi3bp` on every image, so `["raspi3bp"]` matches on every board and any other name never matches; missing means every board. Matching by the running board comes with PR [#659](https://github.com/redox-rpi/redox-rpi/pull/659) |
| `ipv4` | `"dhcp"`, `"static"`, `"off"` (default `"off"`) | `"dhcp"` and `"off"`; `"static"` gives one line "not supported yet" |
| `ipv6` | `"slaac"`, `"static"`, `"off"` (default `"off"`) | accepted, not used yet |
| `address` | `"192.168.1.20/24"` | later, with `ipv4 = "static"` |
| `router` | `"192.168.1.1"` | later |
| `dns` | `["192.168.1.1"]` | later |

Any other key is an error: dhcpd prints one line and starts nothing from
the file, so a key from a newer image is not silently ignored.

## Interface names

netstack names the wired interfaces `eth0`, `eth1`, … in the order it finds
them, and Wi-Fi `wlan0`, …. `eth0` is the first wired adapter on every
board: the LAN7800 on the Pi 3B+, GENET on the Pi 4 and Pi 400, a USB
adapter on the Zero 2 W.

## What starts

- `ipv4 = "dhcp"`: dhcpd asks for an address on that interface once its
  link is up (#560): within 60 s, else one line and quiet retries every
  30 s; a cable plugged in again starts over.
- `ipv4 = "off"`, or no section for the interface: nothing.
- No file, or no interface with DHCP: dhcpd exits at once, without a line.

The file is read once, when dhcpd starts. A change needs a reboot (or
`dhcpd` started again by hand).

## Defaults

- Every image built from `config/raspi3bp-sdcard.toml` (the 3B+ card and
  network-boot images, the Zero 2 W image, the Pi 400 images): `eth0` with
  `ipv4 = "dhcp"`.
- The minimal console card (`scripts/console-image.sh --minimal`, the
  open-firmware release card): no file (it removes `etc/net`), so no
  network starts.

## Examples

No DHCP on this image: remove the file, or set

```toml
[interface.eth0]
ipv4 = "off"
```

IPv6 only, for the smart home node (later, needs the IPv6 work):

```toml
[interface.eth0]
ipv4 = "off"
ipv6 = "slaac"
```

## Upstream files

`/etc/net/ip`, `ip_subnet`, `ip_router` and `dns` stay as in upstream Redox:
netstack reads them once at start as defaults. `config.toml` does not
replace or rewrite them.
