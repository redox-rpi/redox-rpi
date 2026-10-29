# Experiment: lsusb and hwinfo

Issue [#35](https://github.com/redox-rpi/redox-rpi/issues/35), planned for
v0.7.0 (ROADMAP wish list: `lsusb` and hardware info).

```bash
scripts/build-image.sh --experiment usb-hotplug --experiment hwinfo
```

- **Base patch** (numbered at merge; after the experiment `usb-hotplug`,
  agreed with the USB session): `dwc2d` keeps what it learns during
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

The package is built for every image (`scripts/build-image.sh`), but only
this experiment installs it.

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
