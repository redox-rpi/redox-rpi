# Baud rates through termios (experiment `termios-speed`)

Issue: [#94](https://github.com/redox-rpi/redox-rpi/issues/94). On Redox,
relibc's `cfsetispeed`, `cfsetospeed` and `cfsetspeed` always failed with
`EINVAL`, and `cfgetispeed`/`cfgetospeed` always returned 0: only the Linux
branch was written. A program that sets a baud rate the normal way
(`cfsetspeed`, then `tcsetattr`) could not use the USB serial adapters
(#50); only `/scheme/usb-serial/<ports>/baud` worked.

- **relibc `0001`:** Redox keeps one speed for both directions in the low
  5 bits of `c_cflag` (`B0`–`B38400` = `0o0`–`0o17`, `B57600`–`B4000000` =
  `0o20`–`0o36`; no other flag uses them). That is where ptyd and the USB
  serial driver in `dwc2d` read it. The four functions now read and write
  those bits; `cfsetispeed(B0)` keeps the output speed (POSIX). Only
  `src/header/termios/mod.rs` changes.
- `tools/termios-check` (package `termios-check`, linked statically against
  this relibc like `sigregs`): `termios-check <device> <speed>` sets the
  speed with `cfsetspeed` + `tcsetattr`, reads it back with `tcgetattr` +
  `cfgetospeed`/`cfgetispeed`, and checks that a bad speed gives `EINVAL`.

Build on branch `termios-speed` (on `usb-serial`): `--experiment
termios-speed`. `scripts/test-usb-serial.sh` runs the termios checks when
the image has `termios-check`.

## Tests

- QEMU: see below.
