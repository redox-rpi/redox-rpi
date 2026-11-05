# Open-source VPU firmware (experiment)

Can the Pi 3B+ start without Broadcom's closed firmware (`bootcode.bin`,
`start_cd.elf`)? This experiment tries Kristina Brooks's
[rpi-open-firmware](https://github.com/christinaa/rpi-open-firmware), an
open `bootcode.bin` for the VideoCore IV processor (VPU). It lives in its
own repository, `redox-rpi/rpi-open-firmware` (GPLv2+, like upstream; not
created yet, waiting for the maintainer's OK), so this repository stays MIT.

## State

- Nothing on `main` uses it. The Pi still boots with the closed firmware.
- **Upstream:** GPLv2+ with BSD-licensed Broadcom headers. On hold since
  2019 (last commit 2019-02-04). It was developed on the Pi 2 and also runs
  on the Pi 3. In 2018 a user reported that a 3B+ got as far as a Linux
  kernel panic, which means firmware and chainloader both worked
  ([upstream issue #31](https://github.com/christinaa/rpi-open-firmware/issues/31)).
- **What it does:** the VPU part sets the VPU clock (PLLC, 500 MHz), the
  UART (PL011 on GPIO 14/15, 115200 baud), the SDRAM (LPDDR2, 128 MB to
  1 GB, at 400 MHz), the USB power domain and PHY, and the ARM clock
  (PLLB). It then starts the ARM with a small **32-bit** chainloader. The
  chainloader reads `cmdline.txt`, `rpi.dtb` and `zImage` from the SD card's
  FAT partition and jumps to `zImage` like a 32-bit Linux kernel.
- **What it does not do:** no `config.txt`, no `kernel8.img`, no 64-bit
  start, no HDMI or framebuffer, no mailbox property interface (nothing
  runs on the VPU to answer it), no network boot, no USB or Ethernet on
  the ARM side, no clock or voltage management, and no start for the other
  three ARM cores.
- **Build (2026-11-04):** it builds on homenodellm as a normal user with
  `redox-rpi/build-toolchain.sh` and `redox-rpi/build.sh` in that repository.
  The toolchains are VC4 GCC 6.2.1 from
  [itszor/vc4-toolchain](https://github.com/itszor/vc4-toolchain) and
  ARM GCC 13.3 with newlib, built from source with fixed checksums, about
  40 minutes once. `bootcode.bin` is 82 KB (sha256 `f22e42e8…`).

## In progress

- **Pi test #224 (2026-11-05, real Pi 3B+): the open firmware starts.**
  Card swap by the maintainer. The Pi test agent read the serial log; the
  log is on homenodellm in `~/pitest-tools/t224/serial-openfw.log`.
  - Works: the VPU starts and prints on the serial console (`Booting
    Raspberry Pi....`). The SDRAM is set up (`Elpida 1GB LPDDR2`, memory test
    passed from `0xC0000000` to `0xFFF00000`). The USB power domain and PHY
    come up. The ARM clock is set, and ARM core 0 starts the 32-bit
    chainloader in Supervisor mode. `SDRAM_SIZE=0` in the log is right: 0
    means 1 GB in this code.
  - The SD card is found: a 64 MB standard-capacity card (`SD064`, CSD 1.0,
    not SDHC). After the switch to the data clock ("25 MHz"), one read
    gave a FIFO error (word 113, status `0x21`).
  - Then the chainloader stopped with `Fatal Exception: Undefined
    instruction` while reading the MBR. `cmdline.txt`, `rpi.dtb` and the test
    payload were never reached. In the exception dump, `sp` and `lr` are the
    unused user-mode registers, not real values.
  - **Cause of the crash: our compiler, not the Pi.** Two C++ functions in
    the chainloader end without `return`. Upstream built them with an old
    GCC, where that happened to work. GCC 13 puts a trap instruction (`udf`)
    there, and `read_block<Mbr>` hit it at `0xbfdc`. A second bug hid the SD
    error: the read function declared its error variable twice, so it
    always reported success.
- **Fixed in `redox-rpi/rpi-open-firmware` (2026-11-05):** both `return`s
  added, read errors reported. On an error the SD driver now halves the data
  clock and reads again (up to 4 times), and logs the new divider. One boot
  then shows whether the clock was too fast. Why the clock might be too
  fast: the driver assumes a 250 MHz core clock, while `romstage.c`
  programs PLLC differently, so the real SD clock is not known. New card
  image for Pi test #225 (`bootcode.bin` sha256 `9d02c747…`). The build is
  repeatable but not bit-identical, because the firmware prints its build
  time.

## Pi 3B+ (BCM2837B0) compared with the 3B: risks

Read from the code; #224 confirmed the boot ROM, SDRAM and clock points on our Pi:

- **Boot ROM order.** The ROM first looks for `bootcode.bin` on the SD card.
  Only when there is none does it try USB or network boot (on the 3B+ that
  is enabled at the factory). Our card has the open `bootcode.bin`, so the
  ROM runs it. If it hangs, the ROM does not fall back to anything else: the
  Pi just stays silent. **No risk of bricking:** the ROM is fixed, and the
  open firmware never writes the OTP memory. Putting the network-boot card
  back in restores everything.
- **SDRAM.** The 3B+ has the same kind of 1 GB LPDDR2 chip as the 3B. The
  code recognises 1 GB (MR8 = `0x58`) and runs it at 400 MHz with fixed
  timings. That is slower than the closed firmware's 450 MHz (3B) or 500 MHz
  (3B+), so the timings have some margin. Two points are specific to the
  chip revision:
  - The pad slew setting depends on the VPU chip ID (`(cpuid >> 4) & 0xFFF
    == 0x14`). It is not known which branch the B0 chip takes.
  - The RAM maker list has no entry for Micron (maker ID `0xFF`); newer 3B+
    boards use Micron chips. This only changes the log line (`Unknown`), not
    the setup.
  If the memory test fails, the log shows `SDRAM self test failed!`.
- **Clocks and voltage.** The VPU runs at 500 MHz and the ARM at roughly
  470 MHz (PLLB about 939 MHz, divided by 2), well below the 3B+'s
  1400 MHz. That is safe at the power-on core voltage. On the 3B+ a PMIC
  (MaxLinear MxL7704) supplies the voltages; the open firmware never talks
  to it, so the voltages stay at their defaults. The thermal and
  undervoltage monitoring that the closed firmware does is missing.
- **SD card.** The chainloader drives the sdhost controller on GPIO 48–53.
  #224 used a 64 MB standard-capacity card: the card was found, but a read
  at the data clock failed (see above). A common SDHC card (4–32 GB) is the
  other case to try.
- **LAN7515 (USB hub and Ethernet).** The firmware turns on the USB power
  domain and the PHY, but nothing on the ARM side drives USB yet. With the
  closed firmware, the MAC address comes from OTP, either through the
  mailbox (U-Boot) or the devicetree that the firmware fills in. Without
  that, the LAN7800 driver would get no MAC address, so a fixed or random
  one would be needed. U-Boot's network boot also uses the mailbox.
- **Serial console.** The firmware puts the full PL011 UART on GPIO 14/15.
  With the closed firmware, that UART normally serves Bluetooth on the 3B+.
  Our cable is on the same pins, and the baud rate (115200) matches
  `redox-serial`.

## Next: what Redox would need on top

1. **Start the ARM in 64-bit mode.** The VPU code already defines the bit
   (`ARM_C0_AARCH64`) but never sets it. The chainloader is 32-bit and
   switches to non-secure mode early, so it cannot switch to 64-bit later.
   We need a 64-bit start stub that sets up the exception levels and the
   timer frequency, and that parks cores 1–3 in a spin table at `0xd8`
   like Raspberry Pi's `armstub8`. That stub could be Rust; the VPU part
   cannot be, because there is no Rust or LLVM backend for VideoCore IV.
2. **Load `kernel8.img` or U-Boot** instead of `zImage`: U-Boot first,
   because Redox already boots through it.
3. **Replace the mailbox.** `rpi-firmwared`, `rpi-fbd`, the CPU clock
   governor and U-Boot all ask the closed firmware for things like memory
   size, clocks, the MAC address and the framebuffer. With the open firmware
   either the VPU keeps a small mailbox service running or the ARM side
   sets these up itself (devicetree values, a fixed clock, no framebuffer).
4. **USB and Ethernet** for network boot (LAN7515 behind dwc2), and HDMI
   much later (see [graphics.md](graphics.md), `vc4d`).

**Compare with librerpi first.** Its successor project
[librerpi/lk-overlay](https://github.com/librerpi/lk-overlay) (GPLv2+,
still active, last push 2026-08) rewrote the VPU firmware on top of the
little kernel (LK). It has a `rpi3-bootcode` build, an aarch64 ARM stage for
the Pi 3, and composite video. Its README marks booting Linux, Ethernet
(with the MAC address taken from the serial number), USB and SD as working
on the Pi 2 and unknown on the Pi 3. It needs Nix to build. It may be a
better base than the 2019 code for steps 1–4. A second card in the same
visit could test its `bootcode.bin`.

## Later

- Tell the community what we found, once #224 has run (the maintainer
  posts; sessions write notes only).
