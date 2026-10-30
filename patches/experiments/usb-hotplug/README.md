# USB hotplug (experiment `usb-hotplug`)

[#17](https://github.com/redox-rpi/redox-rpi/issues/17). Applies after the
default patches (base `0020`, `0022`, `0023` from `dwc2-default`, on main
since 403aac5).

- **base `0030`:** `dwc2d` asks the root port and every port of every hub
  for connection changes once a second. A new device is reset and
  enumerated (a hub with everything behind it), a removed one is forgotten
  and its address reused.
  - A keyboard plugged in later is used if there is none.
  - All network adapters are kept, but only one receives. The network
    stack sees the best by link: an RTL815x with a link, else the LAN7800,
    else an RTL815x without a link, else CDC Ethernet. It moves to another
    adapter when one is plugged in or removed or the link of an RTL815x
    changes. The adapter that takes over gets the MAC address the stack
    started with, so the stack and its DHCP lease keep working.
  - Adapters plugged in later do not wait for their link at all; the
    rescan looks at it. At boot they wait up to 10 s as before.
  - Without any adapter at boot the scheme is still registered, as
    `network.usb` with the board's address, so an adapter plugged in later
    works (Zero 2 W).
- **base `0031`:** `dhcpd` no longer exits when the server does not answer.
  It waits 5 s for each answer and tries again 2 s later until it is
  configured, and reports only the first failure. A cable or adapter
  plugged in after boot gets an address.

- **base `0032`:** a test command for hotplug without a person: write
  `network`, `keyboard` or `<hub address> <port>` to `/tmp/dwc2d-cycle`,
  and the next rescan switches that hub port off and on. A hub that powers
  its ports only together (ganged) keeps the device connected; dwc2d then
  forgets and resets it, which runs the same code without the electrical
  removal. The log says which one happened, and the hub line says
  `power switching per port` or `ganged`. On the real Pi:
  `scripts/pi-hotplug-test.sh <pi address>` cycles the network adapter's
  port three times and checks the log and devshd (`KEYBOARD=1` adds the
  keyboard's port).

Build: `scripts/build-image.sh --experiment usb-hotplug`.

QEMU check: `scripts/test-hotplug.sh [image]` boots twice. First it
unplugs the USB keyboard, plugs in a new one and types on it, then plugs
in a second network card, unplugs the first and reaches devshd through the
second. Then it boots without a network card and plugs one in later: dhcpd
must configure it. `HOTPLUG_PORTS="<a> <b>"` picks the host ports when
12323 is taken by another QEMU.

**Result on a real Pi 3B+ (2026-10-29, test #35,
`v0.3.0-dev-148-g54f7433+x657a`): pass.** Standard checks pass (0 of 300
starts failed). The Pi's hubs switch power per port: 3 of 3 power cycles
of the LAN7800's port, network back after 7–10 s. The keyboard was
unplugged and plugged into 4 ports, found each time. With the cable pulled
during boot, dhcpd configured the network after 4 attempts
([report](../../../docs/test-reports.md)).

## Merge to main

Merge-ready (2026-10-30): main is merged in, and the three patches apply
unchanged after main's base patches (rebuilt on `0024` and `0030`, same
patch IDs as before).

- **Numbers at the merge:** `git mv` to `patches/base/` in this order:
  `0030` hotplug → `0031`, `0031` dhcpd → `0032`, `0032` cycle command →
  `0033`. No kernel or relibc patches. Then drop `--experiment
  usb-hotplug` from the branches that use it (`dwc2d-boottime`,
  `usb-serial`, `usb-storage`, `usb-mouse`, `usb-storage-more`).
- **Proven on the Pi:** test #35 (`v0.3.0-dev-148-g54f7433+x657a`:
  standard checks, 3 power cycles of the LAN7800's port, keyboard in 4
  ports, cable pulled during boot) and test #67
  (`v0.3.0-dev-176-g752ac70+x8057`, with `dwc2d-boottime` on top, which
  carries the later halt and SETUP fixes in these same patches):
  `scripts/pi-hotplug-test.sh` 15/15 twice, keyboard cycle.
- **QEMU after the merge with main:** `scripts/test-hotplug.sh` 15/15 with
  `dwc2d-boottime` (`v0.3.0-dev-325-g9c7ecdb+x8057`) and with `usb-serial`
  (`v0.3.0-dev-332-gbf1ec19+x41ea`) on top.
- **Still needs a person:** nothing for the merge. The Anker hub steps
  stay with test #4 (rtl8152 session).
