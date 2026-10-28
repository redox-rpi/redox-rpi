# Open tasks

Small, self-contained tasks for contributors and their AI agents. Before you
start, read [CONTRIBUTING.md](../CONTRIBUTING.md) and claim the task in its
issue. "Needs" says which hardware the task requires: **QEMU** tasks run on
any Linux build host, **Pi** tasks need a Raspberry Pi 3B+, usually with a
USB-serial adapter on its console.

## Stability

### T1: Measure the remaining start crashes in QEMU (QEMU)

Dynamically linked programs still fail at start: about 2 of 100 starts in
QEMU, 36 of 300 on a real Pi with relibc patch `0005` (`ld.so` reads a
malformed ELF file, or `brk #1` with "no filetable fd present"). With the
`ldso-diagnostics` experiment, which reads every byte instead of one per
page, the Pi had 0 of 300 ([test reports](test-reports.md)).

- **Do:** Build the default netboot image and run the start loop from
  [debugging-random-crashes.md](debugging-random-crashes.md) for at least
  1000 starts. Count failures and collect the crash dumps from
  `images/qrun.log`. Also try an experiment that reads every 64th byte
  (one per cache line) instead of every byte or one per page.
- **Done when:** a table with the rates, the build IDs and the crash
  addresses is in the task's issue.

### T2: Find the kernel cause of the start crashes (QEMU)

Relibc patch `0005` (touch fresh memory before copying into it) lowers the
crash rate but does not fix it. The lazy borrow path was ruled out
(experiment `kernel-borrow-populate`).

- **Look at:** page fault handling for anonymous memory in
  `src/context/memory.rs` of the kernel, what happens when a page is
  populated by a write fault from a scheme call in another context.
- **Done when:** an experiment patch that brings the rate to 0 in 1000
  starts without relibc `0005`, with an explanation of why.

### T3: Instruction cache maintenance after loading code (QEMU, then Pi)

Nothing on aarch64 cleans the data cache and invalidates the instruction
cache after `ld.so` or `exec` copied code into memory. QEMU does not need
this; a real Cortex-A53 can execute stale instructions.
`SCTLR_EL1.UCI` is not set, so user space cannot use `dc cvau`/`ic ivau`.

- **Do:** an experiment that does the maintenance, either in the kernel
  (for example when a page becomes executable) or by setting
  `SCTLR_EL1.UCI` and doing it in `ld.so`.
- **Done when:** the experiment builds, the QEMU start loop is not worse,
  and the result on a real Pi (if you have one) is in
  [test-reports.md](test-reports.md).

### T4: Signal mask bug in the aarch64 signal entry (Pi)

In relibc's `redox-rt/src/arch/aarch64.rs`, the code at label `3` in
`__relibc_internal_sigentry` stores the signal number where the signal
word belongs, so after one `pthread_kill` all signals of that thread stay
blocked. Shown in QEMU by `sigregs thread`
([test reports](test-reports.md)); experiment `sigentry-thread-signals`
fixes it and passes in QEMU and on a real Pi.

- **Do:** review the patch against the x86_64 version of the same
  function (x86_64 also adds 64 to the signal number of signals sent to a
  thread; aarch64 does not) and decide whether it becomes relibc patch
  `0007`.
- **Done when:** reviewed, and either moved to `patches/relibc/` or the
  reason it stays an experiment is in its README.

### T5: Programs exit with code 1 and print nothing (QEMU or Pi)

`ps` straight to the terminal often exits with code 1 and no output;
`ps | cat` and `ps > file` work. `reboot` showed the same: exit code 1, no
message, it took several attempts. Seen over devshd on the Pi.

The serial console shows the cause: these are start failures, the same as
T12. On a real Pi 3B+, `reboot` crashed twice and `ps` once, each at
`relibc_start_v1+0xB34`, which is the `brk #1` after
`expect("no filetable fd present")`. Details in T12.

- **Look at:** T12. What is left here: why `ps` straight to the terminal
  fails much more often than `ps | cat`.
- **Done when:** T12 is fixed and `ps` and `reboot` work every time.

### T11: devshd sessions leave `login` processes behind (Pi or QEMU)

After about 30 devshd sessions, 31 `login` processes were still running.

- **Look at:** `tools/devshd` (how it ends a session and waits for its
  child) and `login` in userutils.
- **Done when:** `ps | cat` shows no extra `login` after 20 sessions with
  `scripts/devsh.py`.

### T12: Programs fail at start with "no filetable fd present" (QEMU, Pi)

A dynamically linked `sigregs` dies before `main` in about half of the
starts in QEMU and in every start on a real Pi: `relibc_start_v1` stops at
`.expect("no filetable fd present")` in `src/start.rs` (`brk #1`,
exception class `0x3c`, nothing printed, exit code 1). The thread fd from
the same auxv is there, the filetable fd is not. `ld.so` found it just
before (`src/ld_so/start.rs` has the same check). Likely the same as the
`brk #1` crashes in T1. Linked statically, `sigregs` starts every time.

**Confirmed on a real Pi as the cause of T5.** These dumps were recorded on
the serial console (`logs/serial.log` on the build host, archived in
`logs/serial/` since 2026-10-28):

| Program | Image | ELR | `X17` (PLT target) |
|---|---|---|---|
| `reboot` | `v0.1.0-alpha-14-gf39d20b+ldso-diagnostics` | `0x1C8E08` | `0x1C82D4` |
| `reboot` | the same image, after a new boot | `0x1C8E08` | |
| `ps` | `v0.1.0-alpha-52-ga4ba2e3+ldso-diagnostics+sdhost-driver+sdhost-boot` | `0x1D8E08` | |

- **Where:**
  - All three have `ESR_EL1` `0xF2000001` (`brk #1`) and the same page
    offset `0xE08`, so the crash is in `libc.so`.
  - `X17` is `relibc_start_v1`, so the crash is at
    `relibc_start_v1+0xB34`. In the `libc.so` from the build, that address
    is the `brk #1` after
    `expect_failed("no filetable fd present")` (23 bytes).
  - The link register points into the program's own start code.
- **Not hidden by `ldso-diagnostics`:** both images have that experiment
  and still crash here.
- **Depends on how the program starts:**
  - In the same session, `ls` and `pwd` started 100 times out of 100.
  - `ps` straight to the terminal fails most of the time, while
    `ps | cat` rarely fails.
  - This points to the way the program is started, more than to random
    memory corruption.

- **Do:** reproduce with the dynamically linked `sigregs` (build it with
  `template = "cargo"`, see `scripts/build-image.sh`), find out whether the
  auxv on the new stack is incomplete when `ld.so` hands over to the
  program, or whether `relibc_start_v1` reads it from the wrong place.
  An experiment that prints the auxv in `ld.so` before the handover and in
  `relibc_start_v1` before the `expect` shows this on the serial console.
  Compare `ps` with `ps | cat`.
- **Done when:** the cause is known, with an experiment patch that lets the
  dynamically linked `sigregs` start 100 times out of 100 on the Pi.

## Hardware

### T18: No `pcid` and `audiod` on the Pi (QEMU, then Pi)

The Pi 3B+ and the Zero 2 W have no PCI bus and never will, and there is no
sound driver yet. Still, every boot starts `pcid-spawner`, whose `pcid`
panics ("not yet implemented: Pci::CfgAccess::read on this architecture"),
and `audiod`, which stops with "No such device". Both fill the console with
errors that hide real ones. No PCI detection is needed: leave them out of
the Pi images.

- **Look at:** in base, `init.initfs.d/40_pcid-spawner-initfs.service` and
  its line in `40_drivers.target`, `init.d/00_pcid-spawner.service` in
  `00_base.target`, the `audiod` service, and `init.d/10_smolnetd.service`,
  which waits for `pcid-spawner`.
- **Do:** an experiment with a base patch that drops these services, on a
  branch. `audiod` comes back with a sound driver (Audio on the
  [wish list](../ROADMAP.md#wish-list-help-wanted)).
- **Done when:** a Pi 3B+ boots without these messages, network, USB
  keyboard and `scripts/pi-test.py` work as before, and the result is in
  [test-reports.md](test-reports.md).

### T21: CPU clock and throttling watch through the firmware (Pi)

The Pi runs at 600 MHz of its 1.4 GHz: nothing asks the firmware for a
higher ARM clock. Linux does this with `raspberrypi-cpufreq`.

- **Do:** on branch `cpu-clock` (experiment `cpu-clock`), let
  `rpi-firmwared` set the ARM clock with the firmware's "set clock rate"
  call (tag `0x38002`). Also poll `throttled` (tag `0x30046`) and log a
  warning when undervoltage or overheating is reported, as Linux'
  `raspberrypi-hwmon` does: a higher clock means more heat and current.
- **Done when:** the Pi runs at its maximum ARM clock, the start loop and a
  build or test time are measured before and after, a throttling warning is
  shown when it happens, and the result is in [test-reports.md](test-reports.md).
  Driver docs: [rpi-firmware.md](rpi-firmware.md).

### T6: sdhost SD driver on the real Pi (Pi with serial console)

Done 2026-10-27: with `sdhost-driver` and `sdhost-boot` the Pi boots with
sdhost and writes correctly ([results](../patches/experiments/sdhost-boot/README.md#results)).
Open: read the real core clock from `/scheme/rpi-firmware` instead of
assuming 400 MHz (the Pi 3B+ has 250 MHz; 31 MHz card clock now, 50 MHz
possible), see the
[sdhost-driver README](../patches/experiments/sdhost-driver/README.md#results).

### T16: RedoxFS writes 36 times the data for small writes (QEMU, then Pi)

Writing 1 MiB in 4 KiB calls to `/scheme/data` sent about 74,000 blocks
(37 MB) to the SD card; 1 MiB calls write at bus speed
([statistics](../patches/experiments/sdhost-stats/README.md#results)).

- **Look at:** what RedoxFS writes per transaction (node, block list, tree,
  allocator log, header), in the `redoxfs` recipe. Count the writes per
  4 KiB `write` call, in QEMU with `sdhost-stats` or with a counter in the
  disk scheme.
- **Done when:** the writes per small `write` call are explained, with an
  experiment patch that lowers them, or the reason it cannot be lower.

### T17: USB hotplug (QEMU, then Pi 3B+ or Zero 2 W)

A USB device plugged in after boot is not detected, on every board: `dwc2d`
enumerates the devices once at start and does not watch the root port.
Seen with a keyboard on the Zero 2 W ([test reports](test-reports.md)); the
Pi 3B+ is the same, its keyboard and LAN7800 sit behind the onboard hub.

- **Look at:** `dwc2d` in base, the port connect and change interrupts
  (`HPRT0`, `GINTSTS.PRTINT`), and the hub status change endpoint for
  devices behind a hub. Branch `dwc2-interrupts` adds interrupt handling and
  is the place to start.
- **Done when:** a keyboard plugged in after boot types, and unplugging and
  plugging it again works, on a real Pi.

### T7: Power off and reboot (QEMU, then Pi)

`reboot` resets the Pi through the watchdog (kernel patch `0002`).
`shutdown`/power off does not exist yet.

- **Done when:** the Pi stops cleanly, with the power LED state documented.

### T8: Raspberry Pi Zero 2 W board variant (Zero 2 W)

Same SoC as the Pi 3, 512 MB RAM, no onboard Ethernet. See
[ROADMAP.md](../ROADMAP.md), phase for the Zero 2 W.

- **Done when:** the image boots to a login prompt on a Zero 2 W.

State: the unchanged `v0.1.0-alpha` SD card image boots on a Zero 2 W, and a
USB keyboard works when plugged in before boot
([test reports](test-reports.md)). Open:
- **USB hotplug:** see T17, the same on the Pi 3B+.
- **Network:** no driver for the HAT's RTL8152 on `main` yet (experiment
  `rtl8152` on branch `rtl8152`). The unchanged image already sees the chip:
  `0bda:8152` at high speed on port 4 of the HAT's hub (`1a40:0101`), bulk
  endpoints 0x81/0x02 with 512 bytes, as the driver expects. The CDC
  Ethernet search then fails on the chip's second configuration
  (`GET_DESCRIPTOR` config index 1, wLength 512: `Timeout("channel halt")`);
  the experiment claims the chip before that search runs.
- **Network boot:** works on a real Zero with a 128 MiB image and a small
  U-Boot card ([netboot](netboot.md#pi-zero-2-w)), but only over TFTP with
  512-byte blocks and slowly. Faster: [T19](#t19-fast-network-boot-on-the-zero-2-w-zero-2-w-with-the-ethernetusb-hat).

## Boot

### T15: Boot the kernel straight from the Pi firmware (Pi with serial console)

Today the Pi firmware starts U-Boot, U-Boot provides UEFI for the Redox
bootloader, and the bootloader loads the kernel and initfs from RedoxFS.
Raspberry Pi OS skips all of that: the firmware loads the kernel itself
(`kernel8.img`) and the initramfs (`initramfs` in `config.txt`) and passes
the devicetree, which also says where the initramfs is.

- **Do:** an experiment with a small entry stub in the kernel that does what
  the bootloader does today: build the memory map from the devicetree, find
  the initfs through `/chosen` (`linux,initrd-start`/`-end`), set up the
  first page tables if the bootloader does that on aarch64 (check), and call
  the normal kernel start. Plus a `make-sdcard.sh` mode that puts kernel and
  initfs on the FAT partition.
- **Keep:** U-Boot stays for network boot during development.
- **Open questions:** kernel updates then mean writing to the FAT
  partition; how the image updates it.
- **Done when:** a Pi 3B+ boots from the SD card without U-Boot and the
  Redox bootloader to the login prompt, and `scripts/pi-test.py` passes as
  with the U-Boot image.

Without U-Boot, two things it does today must come from the firmware
instead: the Ethernet MAC address (tag `0x10003`, U-Boot writes it into the
devicetree) and the kernel command line from `cmdline.txt` (tag `0x50001`).

### T19: Fast network boot on the Zero 2 W (Zero 2 W with the Ethernet/USB HAT)

U-Boot's r8152 driver receives into a 2048-byte buffer
(`RTL8152_AGG_BUF_SZ` in `drivers/usb/eth/r8152.h`). On a real Zero 2 W
the image transfer over the HAT's RTL8152 fails with
`Rx: failed to receive: -5`, over HTTP and over TFTP with full 1468-byte
blocks. With 512-byte TFTP blocks the whole image arrives, but it takes
many minutes ([test report](test-reports.md), [netboot](netboot.md#pi-zero-2-w)).
Measured on 2026-10-27 at the server: 6 blocks per second, about 3 KB/s,
so the 42 MB image of the `rtl8152` test would have taken 3.5 to 4 hours.
That test was given up during the download.
The likely cause: the adapter packs frames that arrive close together into
one USB transfer, and two full-size frames do not fit. Linux uses 16 KiB.

- **Do:** a U-Boot patch (on a branch, in `scripts/build-uboot.sh`'s patch
  series) that raises the buffer and the matching `USB_RX_EARLY_SIZE`,
  then a new Zero card, and `netboot.sh --board zero2w` back to full blocks
  and HTTP. Check U-Boot `master` first; we build v2026.10.
- **Open questions:** whether DWC2 on the Zero copes with 16 KiB bulk
  transfers (`CONFIG_USB_DWC2_BUFFER_SIZE` is 64 KiB); whether to report
  the bug to U-Boot (mailing list, `u-boot@lists.denx.de`) once confirmed.
- **Done when:** a Zero 2 W loads the image over HTTP and boots to the
  login prompt, with the load time in the test report.

### T9: CI builds with the pinned revisions (no hardware)

`.github/workflows/qemu-boot.yml` builds the latest upstream kernel, not the
commit from [config/upstream.lock](../config/upstream.lock).

- **Do:** use the pinned kernel commit; add a weekly job that tries the
  latest upstream and reports whether our patches still apply.
- **Done when:** both jobs run green on a pull request.

### T13: relibc tests on Redox in CI (no hardware, long builds)

[`relibc.yml`](../.github/workflows/relibc.yml) builds relibc and runs
`sigregs` on Linux, but nothing on Redox. The image build takes too long
for a hosted runner.

- **Do:** a job (self-hosted runner on a build host, or a cached
  toolchain) that builds the netboot image with `--experiment
  sigregs-test` and runs `sigregs` in QEMU with
  `scripts/qemu-raspi3b.sh --run`; then relibc's own test suite
  (`tests/`, `make test` with the Redox toolchain) the same way.
- **Done when:** a pull request that brings back the x1/x2 swap
  (experiment `sigentry-x1x2-revert`) turns the job red.

### T14: Deeper analysis of relibc (no hardware)

relibc is the most critical component: every program uses it, and two
bugs in its aarch64 signal entry went unnoticed (see
[test reports](test-reports.md)). Its aarch64 tests are not run upstream.

- **Do:** one at a time, each as its own pull request:
  - unit tests on the host for code that does not need Redox (string,
    number formatting, `ld.so` ELF parsing), with Miri where it runs;
  - a check in CI that the offsets in the aarch64 signal entry match
    `ArchIntRegs` and `SigStack` (disassemble and compare, as in the
    `sigentry-by-name` README);
  - coverage of the tests in `tests/` on Redox, once T13 runs.
- Work in [redox-rpi/relibc](https://github.com/redox-rpi/relibc)
  (CONTRIBUTING.md, step 4); fixes go back as patches here.
- **Done when:** each part runs in CI.

### T10: Start loop in `pi-test.py` with crash details (Pi or QEMU)

`scripts/pi-test.py` counts failed starts, but does not record which
program failed with which message.

- **Done when:** the report lists each failure with program and message.
