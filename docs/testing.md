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

[`.github/workflows/relibc.yml`](../.github/workflows/relibc.yml) runs on
changes to relibc patches, relibc experiments or `tools/sigregs`, and
weekly:

- builds relibc at the pinned commit with `patches/relibc/`, alone and
  with each relibc experiment, for `aarch64-unknown-redox`, and alone for
  `x86_64-unknown-redox`;
- runs clippy (relibc's own lint settings) and attaches the report;
- runs `sigregs` on an aarch64 Linux runner, to check the test itself.

The same checkout locally: `scripts/relibc-checkout.sh <dir>
[--experiment <name>]`, then in `<dir>`:

```bash
CC_aarch64_unknown_redox=clang cargo build --release --lib \
    --target aarch64-unknown-redox -Zbuild-std=core,alloc
```

Nothing in CI runs on Redox yet: `sigregs` and the relibc test suite on
Redox need the image build (QEMU) or a Pi ([tasks T13, T14](tasks.md)).

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
program starts); with `--report <file>` it appends the results to that file.

The queue for the test Pi and all reports are in the repository
[redox-rpi/pi-tests](https://github.com/redox-rpi/pi-tests):
[queue.md](https://github.com/redox-rpi/pi-tests/blob/main/queue.md),
[reports/](https://github.com/redox-rpi/pi-tests/tree/main/reports) and
[lab.md](https://github.com/redox-rpi/pi-tests/blob/main/lab.md) (the
hardware, the serial console, the stable image and the nightly run).
`scripts/pi-tests-queue.sh` prints the queue for scripts.
[process/test-standards.md](process/test-standards.md) says what a row and a
report contain.

### Regression set before a Pi row

[`scripts/regression-set.py`](../scripts/regression-set.py) runs the checks
of regressions we already had, each named after the issue it guards
([#265](https://github.com/redox-rpi/redox-rpi/issues/265)). A PR that
changes `patches/`, `config/` or recipes runs it in QEMU on the build host
before its Pi row is queued; the row names the result. On the build host, in
your own clone, after `build-image.sh`:

```bash
scripts/regression-qemu.sh
```

It boots the last network-boot build in QEMU (`--usb-net --data`), runs the
set against it and writes `build/regression/<time>/report.md` next to the
console log; `--image <file>` boots another image. Exit 1 means a check
failed: fix it before the row. The checks:

| Check | Guards | Pass |
|---|---|---|
| basics | – | `pi-test.py` standard checks, 50 starts |
| signals-stop | #345 | inputd and logd survive SIGSTOP/SIGCONT with the same PIDs; `/scheme/log` still reaches the console |
| waitpid | #386 | `waitpid` while only default-action signals arrive returns the child, never EINTR (needs the experiment tcc) |
| sig-pipe | #205 | `check.ion` prints `SIGREPRO: pass` (experiments sig-pipe and tcc) |
| scanf-pipe | #338 | `check.ion` passes for its own pipe, a file and a shell pipe (experiments scanf-pipe and tcc) |
| tcp | #364, #92 | 1 MiB (QEMU) or 4 MiB (Pi) from the host with `nc`, same SHA-256 |
| dns | #358, #374 | name server from DHCP; unknown name: "Name does not resolve" within 5 s; dead name server: "Try again" within 15 s; works again after it is set back |
| serial | #92, #151, base 0030 | no panic, `RELIBC PANIC`, `UNHANDLED EXCEPTION` or `main loop stuck` in the console log; no pcid or rtcd |

A check whose experiment is not in the image is skipped, not failed. On the
Pi the same script runs against devshd (`scripts/regression-set.py <pi>
--serial-log ~/redox-rpi/logs/serial.log --dead-dns <unused address>`).
QEMU does not see hardware: the audio rate, real USB and the 3B+ external
abort stay with `pi-test.py --extended` and the Pi rows. A new regression
gets a check here at the cheapest level that sees it.

### Building on the build host

`scripts/build-image.sh` waits for any other build in the same clone to
finish (lock file `build/.build-image.lock`). Run other jobs that use the
build output, such as a QEMU test, under `scripts/with-build-lock.sh
<command>`; it takes the same lock, and a `build-image.sh` inside it does
not lock again. Never wrap a command in `flock` on that file yourself:
`build-image.sh` would wait for it forever. Every build without `--release`
also gets the **test experiments** in `config/test-experiments.conf` (now
`netstack-watch`, for #92 and #217), and the manifest lists them. Benchmark
and A/B rows, where the watch thread would count, build with
`--no-test-experiments`.

## Known problems

- Programs started from the shell crash at random (`no filetable fd present`
  or a TLS index error in relibc). Not Pi-specific; see
  [status](status.md).
