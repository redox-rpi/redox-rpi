# Testing

Redox can be tested without a Pi: QEMU emulates a Raspberry Pi 3
(`-M raspi3b`) closely enough to boot U-Boot, the Redox bootloader and the
kernel, including all four cores and the SD card controller. QEMU does not
emulate the Pi's Ethernet chip.

## Interactive boot

[`scripts/qemu-raspi3b.sh`](../scripts/qemu-raspi3b.sh) works on macOS and
Linux (needs `qemu-system-aarch64`, `curl`, `zstd`) and over SSH.

Latest daily Redox image, with our kernel:

```sh
scripts/build-kernel.sh
scripts/qemu-raspi3b.sh --kernel upstream/kernel/build/aarch64/kernel
```

Self-built image (after `scripts/build-image.sh` on the Linux host):

```sh
scripts/qemu-raspi3b.sh --image upstream/redox/build/aarch64/minimal/harddrive.img
```

What happens:

1. U-Boot starts. Do not press a key during its countdown, otherwise it
   stops at its own `U-Boot>` prompt (type `boot` to continue).
2. The Redox bootloader shows a resolution menu: press **Enter**.
3. The kernel log follows, including `CPU activation complete: 4 CPUs online`.
4. At `redox login:` log in as `root`, password `password`.
5. Exit QEMU with **Ctrl-A**, then **X** (inside `screen`/`tmux`:
   Ctrl-A, A, X).

The daily server image boots in live mode and needs about 3 minutes in QEMU
on a Mac. Our own image boots from the emulated SD card in about 25 seconds
on the Linux build host.

## Automated boot test

```sh
scripts/qemu-raspi3b.sh --test --kernel upstream/kernel/build/aarch64/kernel \
    --expect "CPU activation complete: 4 CPUs online" --expect "redox login:"
```

Answers the bootloader menu itself, passes once every `--expect` pattern
appeared on the serial console, fails on a kernel panic or timeout. The
console log is kept in `images/qemu-test.log`.

## CI

[`.github/workflows/qemu-boot.yml`](../.github/workflows/qemu-boot.yml) builds
the latest upstream kernel with our patches and runs the automated boot test
on every push, on pull requests and daily. Results:
[Actions](https://github.com/redox-rpi/redox-rpi/actions); the serial log of
each run is attached as an artifact.

## Which U-Boot

If our U-Boot is built (`scripts/build-uboot.sh`, writes
`build/u-boot/u-boot.bin`), the script uses it automatically; otherwise it
downloads Redox's prebuilt U-Boot 2022.07. Choose explicitly with
`--uboot ours` or `--uboot redox`. The first line of the output says which one
runs.

Our U-Boot takes the devicetree from the Pi firmware, which QEMU does not
have. The script therefore passes the mainline devicetree built with U-Boot
and adjusts it the way the `config.txt` overlays do on a real Pi:
`stdout-path` on the PL011 (like `disable-bt`) and the `sdhost` controller
disabled, so the SD card is on the Arasan controller (like `dtoverlay=mmc`).

## Tests on the real Pi

[`scripts/pi-test.py`](../scripts/pi-test.py) runs the standard checks on a
netbooted Pi through devshd (version and manifest, `ps`, SD card, repeated
program starts) and appends the results to
[test-reports.md](test-reports.md).

## Known problems

- Programs started from the shell crash at random (`no filetable fd present`
  or a TLS index error in relibc). Not Pi-specific; see
  [status](status.md).
