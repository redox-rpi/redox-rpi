# USB hotplug (experiment `usb-hotplug`)

[#17](https://github.com/redox-rpi/redox-rpi/issues/17). Applies after the
default patches of branch `dwc2-default` (base `0020`, `0022`, `0023`).

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
