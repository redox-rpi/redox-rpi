# No lost characters on the serial console (#28, was experiment `serial-rx`)

**Default since 2026-10-29:** base `0024` and kernel `0007` (moved to
`patches/base/` and `patches/kernel/`) are in every image. They also fix
the getty panic of #32. `--experiment serial-rx` is still accepted and does
nothing. This page stays as the patches' documentation.

Text sent quickly to the Pi's serial console lost characters: a command
sent in one piece arrived cut off after about 25 characters. Typing one
character every 0.15 s worked.

## Cause

The console is the PL011 UART on GPIO 14 and 15 (`dtoverlay=disable-bt`
in `config/config.txt`). The kernel reads it in the receive interrupt
(`src/devices/uart_pl011.rs`). It writes to it with the `COM1` lock held and
waits until the transmit FIFO is empty before each byte. While a write
runs, the receive interrupt cannot run: it either waits for the lock on
another CPU or is masked on this one.

ion redraws the prompt and the whole line after each key, through
`/scheme/debug`. A line of 30 characters means about 60 bytes, or 5 ms at
115200 baud. The receive FIFO of the BCM2835's PL011 holds 16 characters,
which is 1.4 ms of input. The rest is lost.

## Change

Kernel patch
[`0007`](../../kernel/0007-aarch64-keep-PL011-input-while-the-kernel-writes-to-.patch):
while `send()` waits for the transmit FIFO, it moves received bytes into a
stash of 1 KiB in the `SerialPort`. It does this only while the receive
FIFO is at or above its trigger level (8 of 16), so a few bytes stay in
the FIFO. Those make the receive timeout interrupt come after the write,
and `receive()` hands the stash to the debug scheme first, then the FIFO.

With the input kept, a second problem showed up in QEMU: `getty`
panicked with "failed to write master PTY: Operation would block" and
the console was gone. ptyd refuses input with `EWOULDBLOCK` while the
program (ion) has not read enough of the earlier input, and getty kept
reading the console without forwarding ion's output, so ion stopped
reading. Before, the lost characters hid this.

Base patch
[`0024`](../../base/0024-getty-keep-input-the-PTY-does-not-take-yet-instead-o.patch):
getty keeps the input the PTY did not take, stops reading the console
until it is taken, and keeps forwarding the output meanwhile. While input
is pending it polls with a 10 ms timeout to retry it.

```bash
scripts/build-image.sh --experiment timesync --experiment rtl8152
```

## Test on the Pi

Log in as root on the serial console, then on the build host:

```bash
patches/experiments/serial-rx/serial-input-test.sh 20
```

It writes `echo S01<200 characters>` to `/dev/ttyUSB0` in one piece, 20
times, and looks for the complete output line in
`~/redox-rpi/logs/serial.log`. Done when 20 of 20 are complete.

## Results

- QEMU `raspi3b`, `v0.3.0-dev-64-g00378e4+xe9c4` (kernel patch only):
  5 commands with 200 characters each, sent as one line of 1030
  characters. getty panicked after the login, then ion; 0 of 5 ran, 3 of
  3 runs. The stable image without the patches: 5 of 5.
- QEMU `raspi3b`, `v0.3.0-dev-65-gb6c92b4+xe9c4` (kernel and getty
  patch): 5 of 5 complete, no crash.
- QEMU cannot show the lost characters themselves: its PL011 takes input
  from the host only while the FIFO has room.
- QEMU `raspi3b`, `v0.3.0-dev-94-gb1fcbe1+xe9c4` (merged with `main`,
  patches renumbered): 5 of 5.
- Real Pi 3B+, `v0.3.0-dev-94-gb1fcbe1+xe9c4`, test #22 on 2026-10-28:
  `serial-input-test.sh 20` **20 of 20** lines of 200 characters
  complete at full speed; `scripts/pi-test.py` all checks pass, 0 of 300
  starts failed. [Report](../../../docs/test-reports.md).
