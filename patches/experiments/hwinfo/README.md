# lsusb and hwinfo (was experiment `hwinfo`)

Issue [#35](https://github.com/redox-rpi/redox-rpi/issues/35), planned for
v0.7.0 (ROADMAP wish list: `lsusb` and hardware info). **Default since the
merge of this branch:** the patch is base `0053` in `patches/base/` (after
the `usb-hotplug` patches `0031`–`0033`), and every image installs the
package `hwinfo` (`config/raspi3bp-sdcard.toml`). `--experiment hwinfo` is
still accepted and adds nothing.

- **Base patch `0053`:** `dwc2d` keeps what it learns during
  enumeration and serves it read-only as `/scheme/usb.dwc2/devices`, one
  line per device:

  ```
  address hub port vendor product class usb speed interfaces
  1 0 0 0424 2514 09 0200 high 09.00.02
  4 3 1 0424 7800 ff 0210 high ff.00.ff
  ```

  `hub` and `port` are 0 for the device on the root port; `interfaces`
  lists class, subclass and protocol of each interface. `dwc2d` replaces
  the list after the boot enumeration and after each hotplug scan that
  found a change (two lines in `main.rs`); each open reads the current list.
- **[`tools/hwinfo`](../../../tools/hwinfo)**, package `hwinfo` with two
  programs:
  - `lsusb` lists the devices like Linux's `lsusb`. There is no usb.ids
    database: a short table names the Pi's own chips (hubs, LAN7800,
    LAN9514, RTL8152/8153, QEMU's hub); other devices are described by
    their class or interface classes (`keyboard, HID`). `lsusb -t` shows
    the hub tree, `-v` adds the interfaces.
  - `hwinfo` shows the build and experiments, model, revision, serial,
    memory (ARM and GPU share), firmware, CPU count, ARM clock and its
    maximum, the governor if there is one (#23), other clocks, temperature,
    the throttling flags in words, and the number of USB devices. Sources
    that are missing are left out.

The package is built and installed for every image.

QEMU: [`qemu-check.sh`](qemu-check.sh) runs `lsusb` and `hwinfo` over
devsh, then plugs a second keyboard in and out through QEMU's monitor and
checks that `lsusb` follows.

## Test on the Pi

1. Boot, `scripts/pi-test.py` standard checks.
2. `lsusb`, `lsusb -t`, `lsusb -v`: on a Pi 3B+ the hubs `0424:2514` and the
   LAN7800 `0424:7800`, plus whatever is plugged in (keyboard). With a
   person: plug a keyboard in and out, `lsusb` follows within a few
   seconds.
3. `hwinfo`: Pi 3 Model B+, 1 GiB, 4 CPUs, clocks and temperature like
   `/scheme/rpi-firmware`.

## Results

- **QEMU** (build `v0.3.0-dev-137-gde37ec5+xbec7`, 2026-10-29): `ls
  /scheme/usb.dwc2` shows `devices`, with QEMU's hub (`0409:55aa`, address
  1, root port) and keyboard (`0627:0001`, hub 1 port 1, interface
  `03.01.01`). `lsusb`, `-t` and `-v` print them as above; an unknown
  argument gives the usage. `hwinfo` shows the build, Raspberry Pi 3 Model
  B, 1 GiB (960 MiB ARM, 64 MiB GPU), the clocks, 25.0 °C, no throttling
  and 2 USB devices. Two fixes after this run, in the next build: the CPU
  count included the sum line (5 instead of 4), and QEMU's keyboard showed
  as "QEMU USB device" instead of "keyboard".
- Moved on top of `usb-hotplug` (2026-10-29) at the request of the USB
  session: uses its `parent` field, updates the list on hotplug.
- **QEMU on top of `usb-hotplug`** (build `v0.3.0-dev-149-ga3a4670+x02a9`,
  experiments `timesync usb-hotplug hwinfo`, 2026-10-29): `qemu-check.sh`
  passes. `lsusb` lists hub, keyboard and CDC network card, `lsusb -t`
  puts both behind hub port 1 and 2, `hwinfo` shows 4 CPUs and 3 USB
  devices; a keyboard plugged in through the monitor shows up as address
  4 and goes away again when unplugged.
- QEMU after merging the updated `usb-hotplug` (port cycling, base 0032 there), images `v0.3.0-dev-179-g8370f55+x02a9` and, on usb-hotplug 3b460dc, `v0.3.0-dev-183-g6cb24c3+x02a9`: all checks of `qemu-check.sh` pass again, including the keyboard plugged in and out. The dwc2d patch (now 0034, after usb-storage's 0033) also updates the list after a port cycle.
- **QEMU as default** (base `0053` on `usb-hotplug` 186ced7, default build without experiments, image `v0.3.0-dev-486-gdd28c2f`, 2026-10-31): `qemu-check.sh` 4 of 4. `lsusb` lists hub, keyboard and CDC network card; `hwinfo` shows 4 CPUs and 3 USB devices; the keyboard plugged in through the monitor shows up as address 4 and goes away again.
- **Pi test #125 passes** (default build `v0.3.0-dev-486-gdd28c2f`, 2026-10-31, [report](../../../docs/test-reports.md#2026-10-31-v030-dev-486-gdd28c2f-pr-103-lsusb-and-hwinfo-as-defaults)): standard checks, `lsusb` lists both USB2514 hubs, the LAN7800, a hub and a keyboard plugged in; `-t` and `-v` work; `hwinfo` as expected. No `governor` line: `/scheme/rpi-firmware/governor` comes with the ondemand governor (#23, PR #76), not on this branch yet. `hwinfo` reads its `mode` and `load` lines, the format of PR #76, and shows the line once both are merged.
