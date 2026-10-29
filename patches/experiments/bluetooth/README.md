# Experiment: Bluetooth

Issue [#54](https://github.com/redox-rpi/redox-rpi/issues/54), an outlook
for v0.9.0. This is the first step: talk to the Bluetooth half of the Pi
3B+ Wi-Fi/Bluetooth chip (CYW43455, Bluetooth part BCM4345C0), load its
firmware, and run basic HCI commands. There is no Bluetooth stack yet (no
pairing, L2CAP or profiles).

It builds on the [gpio](../gpio/README.md) experiment (#38), which does
the pin mux, so both go in:

```bash
scripts/build-image.sh --experiment gpio --experiment bluetooth
```

## Hardware

The chip's UART is wired to GPIO 30 (CTS), 31 (RTS), 32 (TX) and 33 (RX).
Our serial console keeps the PL011 on GPIO 14/15 (`dtoverlay=disable-bt`
in `config.txt`). So Bluetooth gets the mini UART (AUX) on ALT5 of those
pins, with RTS/CTS flow control. This is the split of Linux's
`miniuart-bt` overlay. BT_ON, the chip's Bluetooth power line, is pin 0 of
the firmware's GPIO expander (firmware GPIO 128).

The mini UART's baud rate is derived from the core clock. The driver
programs it from the rate `rpi-firmwared` reports, and `bt up` tells it
the current rate again. If the firmware throttles the core clock
(undervoltage, heat), the baud rate drifts and HCI gets garbled. Then
`/scheme/rpi-firmware/throttled` is not 0.

## What is in it

- **Base patch `0045`, `rpi-firmwared`:** `/scheme/rpi-firmware/bt_power`
  (`1`/`0`), the BT_ON line. Pi 3B+ only, like `power_led` and the Wi-Fi
  experiment's `wifi_power`. The three patches touch the same lines; at a
  merge keep all entries.
- **Base patch `0046`, `bcm2835-btuartd`** (new driver in the initfs,
  after `bcm2835-gpiod` and `rpi-firmwared`): the mini UART as the scheme
  `btuart`.

  | Path | Read | Write |
  |---|---|---|
  | `/scheme/btuart/data` | received bytes (waits up to 100 ms, then EAGAIN) | sends the bytes |
  | `/scheme/btuart/baud` | baud rate (115200 at start) | sets it |
  | `/scheme/btuart/clock` | core clock in Hz | sets it, reprograms the baud rate |

  A thread moves received bytes from the 8-byte FIFO into a 64 KiB
  buffer, polling every millisecond. RTS holds the chip off when the FIFO
  is nearly full.
- **Tool `bt`** (`tools/bt`, package from this repository). It talks HCI
  in the UART framing (H4):

  ```
  bt up                 power cycle, reset, load the firmware, reset
  bt info               version, firmware build, address, name
  bt scan [seconds]     LE scan, active (default 5 s)
  bt inquiry [seconds]  classic inquiry (default 5 s)
  bt cmd 1001 [bytes]   any HCI command (hex), prints the answer
  ```

  `bt up` loads the firmware as Linux's btbcm does: Download Minidriver
  (fc2e), the file's 322 Write RAM records (fc4c), Launch RAM (fc4e),
  250 ms pause, reset. The firmware build in the version line goes from 0
  (ROM) to the file's build.
- **Package `bt-firmware`** ([recipe](recipes/bt-firmware/recipe.toml)):
  `BCM4345C0.hcd` from
  [RPi-Distro/bluez-firmware](https://github.com/RPi-Distro/bluez-firmware)
  (pinned commit, checked by SHA-256) and its licence, installed to
  `/usr/lib/firmware/brcm/`. It is under the Cypress licence, so it is
  downloaded at build time and never kept in this repository, like the
  Wi-Fi firmware.
- **`scripts/build-image.sh`:** an experiment's `recipes/` directory
  becomes recipes, the same hunk as on branch `wifi`.
- **`scripts/qemu-raspi3b.sh --mini-uart <chardev>`:** connects QEMU's
  mini UART, which is otherwise unused.

Patch numbers here are placeholders; the merge gives the real ones.

## QEMU

QEMU has the mini UART but no Bluetooth chip.
[`fake-hci.py`](fake-hci.py) plays the chip on a TCP port, and
[`qemu-check.sh`](qemu-check.sh) connects the mini UART to it and runs
every `bt` command. The fake answers like the chip: build 0 before the
upload, 344 after; two LE advertisers; one device in the inquiry. The
check also makes sure the fake got all 323 firmware records and no stray
bytes. QEMU's board is a 3B, so there is no `bt_power` file there; the
real power switch, flow control and the baud rate from the core clock can
only be tested on the Pi.

## Test on the Pi

Serial console only, no wiring. Phones or other Bluetooth devices nearby
make the scans more telling. Steps:

```
cat /scheme/rpi-firmware/bt_power
bt up
bt info
cat /scheme/rpi-firmware/bt_power
bt scan 10
bt inquiry 10
cat /scheme/rpi-firmware/throttled
```

Expected:

- `bt_power` is 0 at boot and 1 after `bt up`.
- `bt up` shows `firmware build 0`, then the record count, then a
  non-zero build and `up`.
- `bt info` gives an address and a name.
- The scans list devices if any are near; an empty list without an error
  is a pass when nothing advertises.
- `throttled` is `0x00000000`.

If `bt up` reports "no answer to HCI command 0c03", the chip is not
talking. Likely causes are power, pins or the baud rate.
