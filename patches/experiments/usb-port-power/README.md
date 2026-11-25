# Root port power cycle in dwc2d (was experiment `usb-port-power`)

**Default now:** the patch (was `0099`) moved to `patches/base/` as base
`0126` and is in every image. `--experiment usb-port-power` is still
accepted and does nothing, so older commands, release options and the
nightly config keep working. This page stays as the documentation of the fix.

Issue [#576](https://github.com/redox-rpi/redox-rpi/issues/576). After a
warm reboot (`reboot` over devsh or the serial console) the Pi 3B+ often
comes up without USB, and so without network:

- Pi test 562: 8 of 30 devsh reboots of the same image logged
  `dwc2d: nothing on the root port` and found no hub, no LAN7800, for the
  whole boot. U-Boot had found `4 USB Device(s)` just before. Stable did
  the same (9 of 39 warm resets in that row).
- Pi test 561 A, a second form: the root port connected at Full speed
  (the onboard hub is High speed), and `GET_DESCRIPTOR` to address 0
  failed with `Transfer(130)` three times.

Pi test 566 showed where the hub is lost: at dwc2d start the root port
is fine in every boot, good and bad (`HPRT0 0x100d`: connected, enabled,
powered, High speed; `PCGCCTL 0`). U-Boot hands over a working port, and the
hub disappears during dwc2d's own init, whose only port-related step is the
core soft reset.

- **base `0126`** (was `0099`):
  - `init_host` logs `HPRT0` and `PCGCCTL` as found (`dwc2d: at start
    …`) and writes `PCGCCTL` 0. When the port is connected, enabled and
    powered, it **skips the core soft reset** (`dwc2d: root port working
    at start, no core reset`) and halts the channels instead; the rest of
    the setup runs as before. Otherwise (QEMU, where U-Boot leaves the port
    in reset; a port with nothing on it) the core is reset as before.
  - In `start_bus`, when the root port stays empty after the 2 s connect
    wait, or its device fails all three enumeration attempts: first a port
    reset without waiting for a connection, then the port power off for
    500 ms and on again, starting over after each. Each step logs `HPRT0`,
    `GUSBCFG`, `GOTGCTL`, `PCGCCTL` and `GINTSTS`, and so does the final
    `nothing on the root port` line.
  - Cost: with nothing on the root port (a Zero 2 W without its hub) the
    boot waits up to about 5 s longer for dwc2d. On a normal boot it costs
    nothing.

Versions on the Pi 3B+ (30 devsh reboots each):

| Pi test | Version | Boots with USB | After the recovery steps |
|---|---|---|---|
| 564 A | without | 18/30 | – |
| 564 B | power cycle only | 16/30 | hub back 0/14 |
| 566 | + end port reset, `PCGCCTL` 0, blind port reset | 18/30 | hub back 0/11 |
| **568** | **v3: no core reset for a working port** | **30/30** | never needed |

Test: 30 devsh reboots of the same image on the Pi 3B+, A (without) and
B (with the experiment), counting boots without the hub. Pass: B has 0,
or the logged registers explain why the power cycle did not help.
