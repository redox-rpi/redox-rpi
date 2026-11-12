# Experiment: vc4d-edid, the monitor's EDID over HDMI DDC

Test only ([#188](https://github.com/redox-rpi/redox-rpi/issues/188),
stage 2b), after `vc4d`: builds vc4d from branch `edid` (redox-rpi/vc4d
`80aa2fe`, on `host-tests`). Instead of `vc4d-diag` (both replace the vc4d
recipe).

At start, before it touches the HVS, vc4d reads the EDID over DDC (I2C
address 0x50 on BSC2, the controller the devicetree's hdmi node names in
`ddc`), base block and first extension, and logs to serial:

- `vc4d: edid: DDC controller at 0x3f805000`;
- `vc4d: edid: raw 000: 00 ff ff ...`: the bytes, 16 per line;
- `vc4d: edid: PASS: <maker> product ... "<name>", EDID 1.3, ...`;
- `vc4d: edid: preferred 1920x1080@60.000 Hz, pixel clock 148.500 MHz, h
  1920 fp 88 sync 44 bp 148 (total 2200), v 1080 fp 4 sync 5 bp 36 (total
  1125), sync ++`, the other detailed timings, the range limits, the
  established and standard modes, the CEA-861 video codes;
- `vc4d: edid: FAIL: ...` with the reason: `no acknowledge` without a
  monitor.

The mode on the screen stays the firmware's. Compare the preferred mode
with what the firmware set (Pi test row 337, `socdump display`: pixel valve
2 at 1920x1080, h 88/44/148, v 4/5/36, 148.5 MHz). `VC4D_EDID=off` skips
the read.

QEMU: `scripts/qemu-raspi3b.sh --device i2c-ddc,bus=i2c-bus.2` puts QEMU's
EDID generator on the DDC bus (raspi3b models the BSCs).

Build: `scripts/build-image.sh --experiment timesync --experiment vc4d
--experiment vc4d-edid` (clean `recipes/other/vc4d` first when another vc4d
commit was built before).
