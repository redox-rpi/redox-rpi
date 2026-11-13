# Boot time: where Redox spends it, and how fast a single app can start (#47)

Draft, 2026-11-12. These are QEMU measurements on the build host, plus one finding from the
kernel source that only a real Pi can confirm. Issue
[#47](https://github.com/redox-rpi/redox-rpi/issues/47); topic page [boot](topics/boot.md).

## How it was measured

- Image: `v0.5.0-dev-108-gf98efe3b+x88fd` (main from 2026-11-12, experiments `cproc-qbe`,
  `netstack-watch` and `tcc`). It was booted the direct way, with no U-Boot and no Redox
  bootloader: `scripts/fw-direct-boot.sh` builds the rpi-boot-stub with the kernel built
  in, plus initfs and live disk as the initramfs (#15).
- QEMU raspi3b with 4 CPUs, run by `tsboot.py`, which writes every serial line with the
  seconds since QEMU started. The scripts are in `scripts/boottime/` on branch
  `boottime-47`.
- `INIT_LOG_LEVEL=DEBUG` on the command line makes init print `Starting <service>`, so
  each service gets a start time. This image does not have the `boot-time` experiment
  (PR #85, parallel init), so init starts the services one after the other.
- Times count from the stub's `starting the kernel` line. Before that line, QEMU takes
  0.1–0.5 s, depending on host load. Each case ran 3 times.
- QEMU is not the Pi. It emulates the CPU (TCG), its UART costs nothing, and its SD card
  and USB are models. The ordering and the ratios carry over to the Pi; the absolute
  numbers do not.

## Today's full boot (QEMU, direct boot)

| Step | Time after kernel start |
|---|---|
| Kernel reaches userspace (`Bootstrap entry`) | 0.75–1.0 s |
| initfs services up, switch to the root filesystem (`switchroot to /usr`) | 5.1–5.9 s |
| `redox login:` on the serial console | 10.1–11.5 s |

Where the time goes, from the debug run:

| What | Time | Note |
|---|---|---|
| Kernel, from `Env:` to bootstrap | about 0.6 s | after the last kernel log line; not measured in detail |
| Each service start | 0.06–0.2 s | about 25 services before the login, so 2–3 s in total |
| `dwc2d` | 1.5 s | waits `BOOT_CONNECT_WAIT` (1 s) for late USB devices; on the Pi with a LAN7515 it waits for the link instead (up to `BOOT_LINK_WAIT`) |
| `rm -rf /tmp` (`00_tmp`) | 1.0 s | a legacy script in the root filesystem's init.d; QEMU only, quick on the Pi |
| `mkdir -m a=rwxt /tmp` | 0.7 s | the same script; both start an extra process (`rm`, `mkdir`) |
| `ion governor.ion` (`15_rpi-governor`) | 1.3 s | starting ion just to read one file; it holds up `audiod` and the login |
| getty on the serial line | up to 0.5 s | asks the terminal for its size (`ESC[6n`) and waits 500 ms when nobody answers (as with QEMU's serial file, or a Pi without a terminal attached) |

## One app instead of the full system (QEMU)

A small static C program (`hello-app.c`, built with the cookbook's cross gcc) prints
`APP READY` and its monotonic time. It runs as an initfs service, and `90_initfs.target`
needs only that app. The variants differ in what the app waits for:

| Variant | The app waits for | Kernel start → `APP READY` (3 runs) |
|---|---|---|
| `v-mini` | relibc runtime only (logd, randd, ipcd, zerod, nulld, ramfs), no live disk | 1.9 / 2.3 / 2.6 s |
| `v-mini-live` | the same, with the 222 MiB live disk in the initramfs | 2.2 / 2.2 / 2.4 s |
| `v-fb` | plus inputd, vesad and fbbootlogd (framebuffer) | 2.1 / 2.1 / 2.5 s |
| `v-sd` | plus rpi-firmwared, lived and bcm2835-sdhostd (SD card) | 2.4 / 2.8 / 2.9 s |
| `v-drivers` | all initfs drivers, dwc2d (USB) included | 5.0 / 5.2 / 5.4 s |
| `v-rootfs` | plus RedoxFS mounted from the live disk | 5.7 / 6.1 / 6.3 s |

- The live disk in the initramfs costs almost nothing here (the stub only marks it reserved).
- The framebuffer costs about 0.2 s, and the SD card driver about 0.5 s.
- USB (`dwc2d`) is the largest single step: about 2.5 s with its wait. An app that needs
  no keyboard and no network should leave it out, or start it after the app.
- Anything in the root filesystem adds the mount plus the root init.d (`00_tmp`, governor,
  getty) on top.

## The kernel log costs about 1.4 s on the Pi

On aarch64 the kernel prints all its `debug!` lines: `src/macros.rs` turns them on for
aarch64 and riscv64. Before the first user process, that is about 240 lines (memory areas
and every devicetree device), or 16.8 KB on the serial console. `uart_pl011.rs` `send()`
waits for an empty TX FIFO before every byte, so at 115200 baud (11.5 KB/s) these
lines take **about 1.46 s** on a real Pi. The whole boot to the login writes about 21.7 KB,
or 1.9 s. QEMU does not show this, because its UART does not slow anything down.

What would help (kernel change, not done):

- print `debug!` only with a build feature or a command-line option;
- or send the kernel log to a ring buffer and the UART only from `info!` up;
- or run the console at a higher baud rate (the Pi's PL011 can do 921600 with a 48 MHz UART clock).

A real-Pi run is needed to confirm the 1.46 s: time from the stub's last line to
`Bootstrap entry`, with and without the debug lines.

## What this means for a direct-boot app on the Pi 3B+

These are estimates; none has been measured on the Pi yet.

- Broadcom firmware to the kernel: about 1.25 s, plus start.elf's devicetree handling
  (figures from the furkantokac.com fast-boot article; our own direct-boot timing request
  is in [#324](https://github.com/redox-rpi/redox-rpi/issues/324)).
- Kernel log on the serial console: about 1.4 s today, close to 0 without the debug lines.
- Kernel and initfs services up to the app: about 2–2.6 s in QEMU for `v-mini` to `v-sd`.
  On the Pi, Redox userspace ran at about half the QEMU time (row 215: about 5–6 s from the
  bootloader to the login, against 10–11 s in QEMU;
  row 215 had parallel init, so this ratio is rough), so roughly 1–1.5 s.
- Total for an SD-card app with today's kernel: **about 4 s**. Without the debug lines:
  **about 2.5–3 s**. With our own firmware (#230), which skips start.elf: about 0.5–1 s
  less.

## Easy wins for the normal image

In order of payoff on the Pi:

1. **Kernel debug lines off: about 1.4 s** (above). New; a kernel change.
2. `dwc2d`: tell init it is ready before it waits for late devices and the link, so init
   starts sdhostd and the root filesystem during the wait: about 1–1.5 s. The `boot-time`
   experiment (PR #85: parallel init, `/tmp` made by init itself) already goes this way,
   but it is not in the default image.
3. `15_rpi-governor`: read `/etc/rpi-firmware/governor` in rpi-firmwared itself, or make
   the service `oneshot_async`, so starting ion does not hold up the login. 1.3 s in QEMU.
   On the Pi it is probably much less, because there the root filesystem is in RAM and
   programs start fast. The `/tmp` script is the same case: 1.7 s in QEMU, invisible on
   the Pi (experiment README, test #53).
4. getty: do not wait 500 ms for a terminal answer on the debug console; use the default
   size there. This only matters when no terminal is attached.

The next measurement worth doing is the same QEMU run with `--experiment boot-time`,
and on the Pi a direct-boot run with the probe app, with and without the kernel debug lines.
