# Known issues

State of the redox-rpi images (v0.2.0) for the Raspberry Pi 3 Model B+ and
Zero 2 W. Newest first
within each group. Details and background in [status.md](status.md), ideas
for contributors in the wish list in [ROADMAP.md](../ROADMAP.md).

## Serious

- **Some programs stop at start** with exit code 1 and no output in
  v0.2.0: most often `ps` printed straight to the terminal (`ps | cat`
  works), `reboot` and `timesync -n`. Run it again. Fixed on `main` by
  kernel patch `0003` ([#12](https://github.com/redox-rpi/redox-rpi/issues/12)),
  in the next release. A much rarer start crash (a data abort, about 1 in
  900 on the Pi) remains: [#1](https://github.com/redox-rpi/redox-rpi/issues/1). The frequent random failures of v0.1.0-alpha (about
  every third command) are fixed since v0.2.0, see
  [debugging-random-crashes.md](debugging-random-crashes.md).
- **Many prebuilt programs did not start:** bash, nano, vim, tmux, ripgrep,
  curl, wget, gawk stopped with a `brk` exception; `find` failed to link.
  Left out of the images. Probably the same cause as the random failures
  fixed in v0.2.0; not tested again yet.

## Annoying

- **No `poweroff`.** `echo shutdown > /scheme/sys/kstop` only halts one CPU
  core. Use `reboot` (as root), then pull the power.
- **A USB device plugged in after boot is not detected** (T17). Plug the
  keyboard in before you power on.
- **The clock needs the network.** The Pi has no battery-backed clock.
  `timesync` sets the time from the last saved time at boot and from
  `pool.ntp.org` once the network is up; without network the time lags.
- **US keyboard layout only.**
- **Boot messages about PCI and audio** (`pcid ... panicked`, `audiod: No
  such device`): the Pi has no PCI bus and no audio driver. Harmless.
- **`ls` prints nothing for a path that does not exist**, instead of an error.

## Limits

- **Pi 3 Model B+ and Zero 2 W.** Pi 3 Model B and Pi 4 are not supported.
  The Zero 2 W has network only with an RTL8152/RTL8153 USB Ethernet adapter
  or hub (experimental driver).
- **No Wi-Fi, no Bluetooth, no audio, no GPU acceleration.** Ethernet and a
  USB keyboard work; other USB devices (sticks, mice) do not.
- **The SD card runs at a reduced clock** (at most 12.5 MHz) to avoid data
  errors: reading and writing are slow, but correct.
- **The remote shell `devshd` (port 2323) is unencrypted** and only in the
  network boot image, not in the SD card image. Do not expose it beyond
  your LAN.
- **Default passwords:** `root` / `password`, `user` without password.
