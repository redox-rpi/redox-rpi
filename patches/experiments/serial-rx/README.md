# Experiment: serial-rx, no lost characters on the serial console (#28)

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
[`0004`](kernel/0004-aarch64-keep-PL011-input-while-the-kernel-writes-to-.patch):
while `send()` waits for the transmit FIFO, it moves received bytes into a
stash of 1 KiB in the `SerialPort`. It does this only while the receive
FIFO is at or above its trigger level (8 of 16), so a few bytes stay in
the FIFO. Those make the receive timeout interrupt come after the write,
and `receive()` hands the stash to the debug scheme first, then the FIFO.

```bash
scripts/build-image.sh --experiment timesync --experiment rtl8152 --experiment serial-rx
```

QEMU does not show the problem: its PL011 takes input from the host only
when the FIFO has room. QEMU checks only that the console still works.

## Test on the Pi

Send a line of 200 characters at full speed to `/dev/ttyUSB0`, 20 times,
and compare what the shell echoes (for example `echo <200 characters>`).
Done when all 20 arrive complete.

## Results

See [test-reports.md](../../../docs/test-reports.md).
