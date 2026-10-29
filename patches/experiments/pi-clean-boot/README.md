# No PCI and audio services on the Pi (#18, was experiment `pi-clean-boot`)

**Default since 2026-10-29:** base `0030` (moved to `patches/base/`) is in
every image. `--experiment pi-clean-boot` is still accepted and does
nothing, so older commands keep working. This page stays as the patch's
documentation.

The Raspberry Pi 3B+ and the Zero 2 W have no PCI bus, and there is no
sound driver for them yet. Still every boot started these services, and
their errors hid real ones in the console:

| Service | Where | What it did on the Pi |
|---|---|---|
| `pcid` (started by `hwd`) | initfs | panics: "not yet implemented: Pci::CfgAccess::read on this architecture", with a register dump of 60 lines |
| `pcid-spawner --initfs` | initfs | "No such device (os error 19)", "failed with exit status: 1" |
| `pcid-spawner` | rootfs | the same again |
| `audiod` | rootfs | "No such device", "exited without notifying readiness" |
| `rtcd` | initfs | nothing: it reads the clock on x86 only (no message) |

The base patch
[`0030`](../../base/0030-Leave-out-pcid-pcid-spawner-audiod-and-rtcd-on-the-R.patch)
- adds `condition_board_not` to init units, the opposite of upstream's
  `condition_board`: the unit does not start when the image is built for
  one of the listed boards (`BOARD`, `raspi3bp` for both Pis);
- uses it in `00_pcid-spawner.service`, `40_pcid-spawner-initfs.service`
  and `20_audiod.service`;
- lets `hwd` start `pcid` only when `BOARD` is not `raspi3bp`;
- runs `rtcd` only on x86, as `ps2d` already does.

The unit files stay in place, so `00_base.target`, `40_drivers.target` and
`10_smolnetd.service` keep their `requires_weak` lines unchanged: init
loads a unit whose condition fails but does not start it, and units that
depend on it do not wait for it. The network stack starts as before.

`audiod` comes back with a sound driver: drop its `condition_board_not`
line (Audio on the [wish list](../../../ROADMAP.md#wish-list-help-wanted)).

Other boot messages on the Pi are not services and stay: kernel warnings
about ACPI, PSCI, the GPIO interrupt controller and the PL031 clock, and
`randd` without an entropy source. See [issue #18](https://github.com/redox-rpi/redox-rpi/issues/18) and the
[boot topic](../../../docs/topics/boot.md).

## Results

- QEMU `raspi3b`, `v0.2.0-14-g5978d1a+x5b60`: none of the errors, network
  up, 40 of 40 starts.
- Real Pi 3B+, `v0.3.0-dev-49-g832e1b5+x6256` (the stable image's
  experiments `timesync` and `rtl8152` plus this one): none of the errors
  in the boot log, network up, `scripts/pi-test.py` all checks pass, 0 of
  300 starts failed.
- Real Pi 3B+, test #5 with `dwc2-interrupts`: all pass, keyboard checked.
- As base `0024`, QEMU `raspi3b`, `v0.3.0-dev-129-g6917241+x9f76`: none of
  the errors. Real Pi 3B+, same image, test #33: none of the errors,
  network up, `scripts/pi-test.py` all checks pass, 0 of 300 starts failed.

Details in [test-reports.md](../../../docs/test-reports.md).
