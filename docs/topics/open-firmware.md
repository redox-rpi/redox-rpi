# Open-source VPU firmware (experiment)

Can the Pi 3B+ start without Broadcom's closed firmware (`bootcode.bin`,
`start_cd.elf`)? This experiment tries Kristina Brooks's
[rpi-open-firmware](https://github.com/christinaa/rpi-open-firmware), an
open `bootcode.bin` for the VideoCore IV processor (VPU). It lives in its
own repository, [redox-rpi/rpi-open-firmware](https://github.com/redox-rpi/rpi-open-firmware)
(GPLv2+, like upstream; our work is on its default branch `redox-rpi`), so
this repository stays MIT. Issue:
[#221](https://github.com/redox-rpi/redox-rpi/issues/221).

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
  image for Pi test #249 (`bootcode.bin` sha256 `9d02c747…`). The build is
  repeatable but not bit-identical, because the firmware prints its build
  time.

## Pi 3B+ (BCM2837B0) compared with the 3B: risks

Read from the code; Pi test #224 confirmed the boot ROM, SDRAM and clock points on our Pi:

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
   timer frequency, and that parks cores 1–3 in a spin table (release addresses `0xe0`, `0xe8`, `0xf0`)
   like Raspberry Pi's `armstub8`. That stub could be Rust; the VPU part
   cannot be, because there is no Rust or LLVM backend for VideoCore IV.
   Base: [`armstubs/armstub8.S`](https://github.com/raspberrypi/tools/blob/master/armstubs/armstub8.S)
   in raspberrypi/tools (BSD licence). It is the stub the closed firmware
   runs before Redox today: timer (`CNTFRQ` 19.2 MHz, 54 MHz on the Pi 4),
   L2 latency, FP/SIMD, SMP bit, EL3 to EL2, cores 1–3 parked in the spin table (`0xe0`, `0xe8`, `0xf0`)
   (what our spin-table kernel patch expects), and the GIC set-up on the
   Pi 4. Use it as it is or port it to Rust. The alternative is TF-A's
   BL31, which gives PSCI (see "Other Pi models").
2. **Load `kernel8.img` or U-Boot** instead of `zImage`: U-Boot first,
   because Redox already boots through it.
3. **Replace the mailbox.** `rpi-firmwared`, `rpi-fbd`, the CPU clock
   governor and U-Boot all ask the closed firmware for things like memory
   size, clocks, the MAC address and the framebuffer. With the open firmware
   either the VPU keeps a small mailbox service running or the ARM side
   sets these up itself (devicetree values, a fixed clock, no framebuffer).
4. **USB and Ethernet** for network boot (LAN7515 behind dwc2), and HDMI
   much later (see [graphics.md](graphics.md), `vc4d`).
5. **3D (graphics.md step 3) needs more from the boot code than the
   framebuffer does:** the V3D power domain switched on, the V3D clock set,
   and much more memory set aside for the GPU. Today that means the full
   `start.elf` with `gpu_mem` well above 16: the cut-down `start_cd.elf`
   we run has no 3D. With the open firmware, the VPU part or a driver on
   the ARM side would do these steps itself (Linux has a `bcm2835` power
   driver for this case).

## Our own firmware ([issue #230](https://github.com/redox-rpi/redox-rpi/issues/230))

The maintainer decided on 2026-11-07 to replace **all** closed pieces
(`bootcode.bin`, `start*.elf`/`fixup*.dat`, `armstub8`) with our own code
rather than adopt rpi-open-firmware or lk-overlay as they are. Both start
on the 3B+, but rpi-open-firmware is old C++ with bugs we had to fix, and
lk-overlay runs a whole RTOS (LK) in stage 1, has dead code and builds with
warnings allowed. Ours is flat C for the VPU, one file per hardware block,
`-Werror`, and each file names its source. Code: redox-rpi/rpi-open-firmware,
branch `bootcode`, directory `bootcode/` (GPLv2+, because the SDRAM, clock
and UART sequences come from rpi-open-firmware).

- **M1 (built 2026-11-07, Pi test #265 queued):** VPU clock, UART, SDRAM
  and a memory test of 32 regions over the whole 1 GB. 5.5 KB.
- **Next:** M2 loads our 64-bit ARM stub, U-Boot and the devicetree from
  the FAT partition; M3 a mailbox service on the VPU; M4 HDMI; M5 network
  boot and what 3D needs (see step 5 above).
- **Disassembly of the closed `bootcode.bin`** (internal, behaviour only,
  not published; the README in `bootcode/` lists each finding with its
  source file): it uses SDHOST, not EMMC, for the card; about a third of
  its register references are USB (USB and network boot); it reads
  `config.txt` for `gpu_mem`, `total_mem`, `start_file` and more; it hands
  off to `start.elf` through a small routine copied into SDRAM after a
  cache flush.

**Video codecs:** the hardware decoders (H.264, MPEG-2/VC-1 with a licence
key) are driven by code inside `start.elf`/`start_x.elf`, reached from
the ARM through VCHIQ/MMAL. Taking that code out and reusing it is not an
option: the Broadcom licence allows the files only unmodified, and the code
depends on the rest of the closed firmware (ThreadX, its memory manager,
VCHIQ). The block's registers are not documented. Without the closed
firmware the choices are software decoding on the ARM (FFmpeg with NEON,
H.264 up to about 720p on a Pi 3) or our own driver for the codec block
from scratch, a project of its own. This is one reason to keep Broadcom's
`start.elf` as an option next to our firmware.

## librerpi/lk-overlay compared (code read 2026-11-05, not built yet)

[librerpi/lk-overlay](https://github.com/librerpi/lk-overlay) (GPLv2+, last
commit 2026-06-20, 252 commits) is the newer rewrite by the same community.
Everything runs on the little kernel (LK), on the VPU and on the ARM.
Its boot chain:

1. **Stage 1** (`vc4-stage1`, our `bootcode.bin`). The SDRAM setup
   (`rpi-ddr2/autoram`) is newer than the 2019 code. Stage 1 loads stage 2
   as `/boot/lk.elf` from an ext2/ext4 partition: partition 2 of the SD card
   (`sdhostp1`), behind a FAT partition that holds only `bootcode.bin`.
   Builds for TFTP, USB and SPI exist too.
2. **Stage 2** (`vc4-stage2`, `lk.elf`). It stays running on the VPU, with a
   small **mailbox server** (`mailbox-property-server`). It starts the ARM
   in **64-bit mode** on BCM2837 (it sets `ARM_C0_AARCH64`) with a built-in
   ARM program. Optionally it uses Raspberry Pi's `armstub8`, which parks
   cores 1–3 in the spin table.
3. **ARM stage** (`rpi3-test`, aarch64 LK with `linux-bootloader`). It
   loads `/boot/Image-aarch64`, `/boot/rpi3.dtb`, `cmdline.txt` and
   `initrd` from the same ext partition and starts Linux. By default Linux
   gets only one core; the `docs/booting-arm` notes list ways to fix that
   (`armstub8`, PSCI in U-Boot or LK).

That covers our step 1 (64-bit, spin table through `armstub8`) and part of
step 3. The gaps for us:

- **Mailbox:** the server answers only a few requests: firmware revision,
  ARM memory, clocks list, throttled, reboot and display notices. Missing
  are board model and revision, MAC address, serial number, clock rates,
  temperature, power states and the framebuffer. `rpi-firmwared` and
  U-Boot ask for some of these, so they would need new tags or changes on
  our side.
- **Network:** the USB Ethernet driver (`rpi-usb-nic`) only knows the SMSC
  LAN9514 of the Pi 2/3B (chip ID `0xec000002`), not the **LAN7515/LAN7800
  of the 3B+**. So no network boot from the VPU on our Pi. Linux or Redox
  would still drive the LAN7800 themselves later.
- **Pi 3B+ itself:** the README marks the Pi 3 as unknown for Linux, SD,
  USB and Ethernet. Nobody has reported a 3B+.
- **U-Boot:** not in the chain yet. It would be loaded as
  `Image-aarch64`, but U-Boot asks the mailbox for the board revision and
  the memory size, and only the memory size is answered today.
- **Build:** officially with Nix, which homenodellm doesn't have. The
  stage-2 build embeds the ARM programs for Pi 1, 2 and 3, so all three
  must be built first. Trying it with our own toolchains is the next step;
  it waits for the maintainer's OK to build that code.

So lk-overlay is the better base for 64-bit and the mailbox. We would add
board/revision/MAC tags for U-Boot and `rpi-firmwared`. For network boot,
U-Boot on the ARM side would bring up the LAN7800 rather than the VPU.

## VPU compiler: fewer dependencies (idea, 2026-11-07)

The VPU code builds with VC4 GCC 6.2.1 from
[itszor/vc4-toolchain](https://github.com/itszor/vc4-toolchain): a GCC and
binutils port from around 2017, plus newlib. It works, but it is a large,
old dependency that we don't control, and old GCC versions get harder to
build with new host compilers. The aim is fewer dependencies and more of
the chain under our control. Not started; nothing waits for it.

- **Step 1, cheap:** freeze the toolchain we have. Fixed sources and
  checksums already exist (`redox-rpi/build-toolchain.sh`); add a fixed
  build environment (a container on homenodellm) so the 40-minute build
  keeps working when the host compiler changes.
- **Step 2, our own small compiler:** a VideoCore IV backend for
  [QBE](https://c9x.me/compile/), with [cproc](https://sr.ht/~mcf/cproc/)
  as the C front end. QBE writes assembler text, and
  [vasm](http://sun.hasenbraten.de/vasm/) already has a VideoCore IV
  module (with vlink as the linker), so the hard part, encoding the
  irregular VC4 instructions (16, 32, 48 and 80 bits long), is done. A QBE
  backend is about 2,000-3,000 lines (its arm64 backend is that size).
  Our firmware is flat C without C++, which is the case cproc and QBE
  handle. The code would be slower than GCC's, which hardly matters for
  boot code.
- **Why not TCC:** TCC has its own assembler and linker, so a VC4 port
  would have to write the instruction encoding and the relocations as
  well as the code generator. More work, for weaker code.
- **What no small compiler gives:** the VPU's 16-lane vector unit (GCC
  doesn't use it either; it is written in assembler) and Rust on the VPU,
  which would need an LLVM backend, a project of its own.
- **When:** step 1 whenever the toolchain build breaks or a new build
  host comes. Step 2 when GCC gets in the way (a compiler bug, or people
  failing to build it), or as a self-contained project for a contributor.

## Register dumps: `socdump` (planned, [issue #224](https://github.com/redox-rpi/redox-rpi/issues/224))

A read-only Redox tool, similar to the Linux tools `pll-inspector` and
`pv_dumper` in [librerpi/rpi-tools](https://github.com/librerpi/rpi-tools)
(GPL-2.0, from the lk-overlay authors; we port the idea, not the code).

- **Why:** the closed firmware leaves its clock, PLL and HDMI settings in
  the registers. A dump under the closed firmware gives reference values
  for the open firmware and for HDMI (`vc4d`, [graphics.md](graphics.md)),
  so the HDMI PHY and PLLH don't have to be guessed. A second dump under
  the open firmware shows what differs.
- **Where:** a new crate `tools/socdump`, not part of `rpi-firmwared`. That
  driver runs all the time and only does the mailbox; `socdump` is run by
  hand, maps the register pages with `physmap`, and only reads.
- **Steps:**
  1. Clock manager, PLLs A–D and H, GPIO pin functions on the BCM2837.
     Check the results against `/scheme/rpi-firmware/clocks`.
  2. Pixel valves, HVS, HDMI. It only reads a block whose clock is on.
  3. Reference dumps at 1080p and 720p on the 3B+ (a Pi test row).
  4. `socdump diff`.
- The rest of the other tools: `edid` (making EDIDs) helps with HDMI mode
  tests. The EEPROM and signing tools are for the Pi 4/5 boot EEPROM.
  `webusbboot` needs a USB device port, which the 3B+ doesn't have.
  [librerpi/vpu-x265](https://github.com/librerpi/vpu-x265) (a 2020 VPU
  vector-unit experiment, no license) doesn't matter for booting.

## Other Pi models (later)

The goal is Redox on the Pi 4, Pi 5 and newer models too, so anything we
build here keeps the chip-specific parts in their own tables:

| | 3B+ / 3B / Zero 2 W | Pi 4 | Pi 5 |
|---|---|---|---|
| SoC | BCM2837(B0) | BCM2711 | BCM2712 + RP1 (I/O over PCIe) |
| Peripherals (ARM address) | `0x3F000000` | `0xFE000000` (low peripheral mode) | from the devicetree |
| Crystal | 19.2 MHz | 54 MHz | 54 MHz |
| First boot code | `bootcode.bin` on the SD card | SPI EEPROM | SPI EEPROM |
| Open firmware | rpi-open-firmware, lk-overlay (tested here) | lk-overlay has some Pi 4 work (not checked) | none known |
| Starting cores, interrupts | spin-table, BCM2836 local mailboxes (our kernel patches) | GIC-400 (GICv2); PSCI only with TF-A as `armstub8.bin` | GIC-400; PSCI from the BL31 that the firmware already contains |

- `socdump`: the register addresses come from the devicetree and the
  tables are per chip, so the Pi 4 and Pi 5 are new tables, not a rewrite.
- [raspberrypi/arm-trusted-firmware](https://github.com/raspberrypi/arm-trusted-firmware)
  (TF-A, ports for the Pi 3, 4 and 5) runs at EL3 before the OS and gives
  **PSCI**: start and stop cores, reset, power off. Upstream Redox starts
  cores only through PSCI and routes interrupts only through a GICv2, which
  is why the 3B+ needs our spin-table and IPI kernel patches
  ([status.md](../status.md)). On the 3B+ TF-A would add little (PSCI
  power-off and core hotplug; there's still no GIC). On the Pi 4 and 5 it
  matters a lot: with PSCI and the GIC-400, upstream Redox's own path fits,
  so these boards may need fewer Pi-specific patches than the 3B+. TF-A's
  BL31 could also take the "cores 1–3" step of the open firmware (Next
  step 1) instead of our own stub.
- Open firmware on the Pi 4/5 means replacing the EEPROM boot stage, not
  a file on the card. That's a separate experiment, done only with a way
  to restore the EEPROM.

## Later

- Tell the community what we found, once Pi test #224 has run (the maintainer
  posts; sessions write notes only).
