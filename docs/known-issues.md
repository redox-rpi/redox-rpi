# Known issues

State of the redox-rpi images for the Raspberry Pi 3 Model B+. Newest first
within each group. Details and background in [status.md](status.md), ideas
for contributors in the wish list in [ROADMAP.md](../ROADMAP.md).

## Serious

- **Programs fail at random.** After login, commands often end without
  output and with exit code 1: `ls`, `cat`, `cmp`, `sha256sum`, `kibi`,
  even `reboot`. Running the same command again may work. Happens in QEMU
  too, so it is not a Pi hardware problem. Under investigation, see
  [debugging-random-crashes.md](debugging-random-crashes.md).
- **Many prebuilt programs do not start at all:** bash, nano, vim, tmux,
  ripgrep, curl, wget, gawk stop with a `brk` exception; `find` fails to
  link. Left out of the images. Possibly the same cause as above.

## Annoying

- **No `poweroff`.** `echo shutdown > /scheme/sys/kstop` only halts one CPU
  core. Use `reboot` (as root), then pull the power.
- **No clock.** The Pi has no real-time clock and there is no NTP yet:
  files are dated 1 January 1970 (or 2026, depending on the tool).
- **US keyboard layout only.**
- **Boot messages about PCI and audio** (`pcid ... panicked`, `audiod: No
  such device`): the Pi has no PCI bus and no audio driver. Harmless.
- **`ls` prints nothing for a path that does not exist**, instead of an error.

## Limits

- **Pi 3 Model B+ only.** Pi 3 Model B, Zero 2 W and Pi 4 are not supported
  yet (Zero 2 W is planned).
- **No Wi-Fi, no Bluetooth, no audio, no GPU acceleration.** Ethernet and a
  USB keyboard work; other USB devices (sticks, mice) do not.
- **The SD card runs at a reduced clock** (at most 12.5 MHz) to avoid data
  errors: reading and writing are slow, but correct.
- **The remote shell `devshd` (port 2323) is unencrypted** and only in the
  network boot image, not in the SD card image. Do not expose it beyond
  your LAN.
- **Default passwords:** `root` / `password`, `user` without password.
