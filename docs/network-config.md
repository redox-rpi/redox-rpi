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
| `boards` | list of board names, e.g. `["raspi3bp"]` | used: the section applies only on these boards (see Board names); missing means every board |
| `ipv4` | `"dhcp"`, `"static"`, `"off"` (default `"off"`) | used |
| `ipv6` | `"slaac"`, `"static"`, `"off"` (default `"off"`) | accepted, not used yet |
| `address` | `"192.168.1.20/24"` | with `ipv4 = "static"`: needed, address and prefix length |
| `router` | `"192.168.1.1"` | with `ipv4 = "static"`: optional, the default route |
| `dns` | `["192.168.1.1"]` | with `ipv4 = "static"`: optional; the first entry is the name server |

Any other key is an error: dhcpd prints one line and starts nothing from
the file, so a key from a newer image is not silently ignored.

## Board names

dhcpd reads the board from the devicetree when it starts (the root
`compatible`), so the same image works on every board. A `boards` entry
matches either a short name or any root `compatible` string:

| Short name | Board | devicetree |
|---|---|---|
| `raspi3bp` | Pi 3 Model B+ (also QEMU raspi3b with our scripts, which pass the 3B+ devicetree) | `raspberrypi,3-model-b-plus` |
| `raspi3b` | Pi 3 Model B | `raspberrypi,3-model-b` |
| `zero2w` | Pi Zero 2 W | `raspberrypi,model-zero-2-w` |
| `raspi4b` | Pi 4 Model B | `raspberrypi,4-model-b` |
| `pi400` | Pi 400 | `raspberrypi,400` |

A SoC name such as `brcm,bcm2711` (Pi 4 and Pi 400) also works.

## Interface names

netstack names the wired interfaces `eth0`, `eth1`, … in the order it finds
them, and Wi-Fi `wlan0`, …. `eth0` is the first wired adapter on every
board: the LAN7800 on the Pi 3B+, GENET on the Pi 4 and Pi 400, a USB
adapter on the Zero 2 W.

## What starts

- `ipv4 = "dhcp"`: dhcpd asks for an address on that interface once its
  link is up (#560): within 60 s, else one line and quiet retries every
  30 s; a cable plugged in again starts over.
- `ipv4 = "static"`: dhcpd sets `address` as soon as the interface
  appears (one line `DHCP: eth0 fixed address …`), link or not, and keeps
  it when the link goes down. `router` and `dns` are used like those of a
  DHCP lease: the first wired interface with a router gets the default
  route and the name server. A missing or malformed `address`, `router` or
  `dns` gives one line and nothing starts on that interface.
- `ipv4 = "off"`, or no section for the interface: nothing.
- No file, or no interface with DHCP or a fixed address: dhcpd exits at
  once, without a line.

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

A fixed address instead of DHCP:

```toml
[interface.eth0]
ipv4 = "static"
address = "192.168.1.20/24"
router = "192.168.1.1"
dns = ["192.168.1.1"]
```

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
replace or rewrite them; a fixed address goes to netstack at run time
(netcfg), not into these files.
