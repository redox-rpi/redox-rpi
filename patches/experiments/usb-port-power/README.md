# Root port power cycle in dwc2d (experiment `usb-port-power`)

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

`init_host` does a core soft reset but leaves the root port power as
U-Boot left it, and `reset_port` only ever sets the power bit. Nothing
takes the port through a disconnect.

- **base `0099`:** in `start_bus`, when the root port stays empty after
  the 2 s connect wait, or its device fails all three enumeration
  attempts, dwc2d switches the port power off for 500 ms and on again and
  starts over, up to twice. Each step logs `HPRT0`, `GUSBCFG` and
  `GOTGCTL`, also in the final `nothing on the root port` line.
  - Cost: with nothing on the root port (a Zero 2 W without its hub) the
    boot waits up to about 5 s longer for dwc2d. On a normal boot it costs
    nothing.

Test: 30 devsh reboots of the same image on the Pi 3B+, A (without) and
B (with the experiment), counting boots without the hub. Pass: B has 0,
or the logged registers explain why the power cycle did not help.
