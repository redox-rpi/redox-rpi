# Status and findings

Last update: 2026-10-24. Upstream revisions checked: redox `4338ac2`,
kernel `6cfaba5`, base `d2ffc4c`, relibc `5bfdd35`, bootloader `cc922ec`,
daily image `redox_server_aarch64_2026-10-24_565`.

## Summary

- Redox's latest daily **aarch64 server image boots in QEMU `raspi3b`** (an
  emulated Raspberry Pi 3) up to the login prompt and a root shell, **after
  fixing a kernel panic** in devicetree interrupt parsing (see below).
- Most programs started from the shell currently crash. This is a general
  bug in today's daily build (it happens on QEMU `virt` too), not a Pi issue.
- The Redox kernel **builds on macOS** (Apple Silicon) in under a minute with
  the pinned nightly Rust toolchain. A full image build still needs Linux.

## What exists upstream for the Pi 3

| Area | State |
|---|---|
| Build system | Board `raspi3bp`, only a `minimal` config (`config/aarch64/raspi3bp/minimal.toml`); `make qemu` knows QEMU `raspi3b` |
| Boot chain | Pi firmware → U-Boot (prebuilt 2022.07 from [redox_firmware](https://gitlab.redox-os.org/Ivan/redox_firmware), last change 2023-12) → Redox UEFI bootloader → kernel |
| Devicetree | Kernel reads the DT that U-Boot passes through UEFI. The prebuilt U-Boot uses its **own embedded Pi 3B DT** (`devicetree: embed`) |
| Interrupts | Drivers for `brcm,bcm2836-l1-intc` and `brcm,bcm2836-armctrl-ic` (kernel `src/arch/aarch64/device/irqchip/`) |
| Timer | ARM generic timer, works |
| Serial | PL011 only (`arm,pl011`). The mini UART (`brcm,bcm2835-aux-uart`) is not supported → `dtoverlay=disable-bt` is required so the PL011 is on GPIO14/15 |
| SMP | Only PSCI is supported. The Pi 3 / Zero 2 W use `enable-method = "spin-table"` → **only 1 of 4 cores** runs |
| SD card | `bcm2835-sdhcid` (in `base/drivers/storage/`) for the Arasan controller `brcm,bcm2835-sdhci`. Only started on images built with `BOARD=raspi3bp` (`condition_board`) |
| USB / Ethernet | No DWC2 driver. Redox USB is xHCI only. No LAN9514/LAN7515/RTL8152 drivers |
| Reset / power-off | Not available (no PSCI node on the Pi) |

## Boot test in QEMU `raspi3b` (macOS, QEMU 11.1.2)

Reproduce with [`scripts/qemu-raspi3b.sh`](../scripts/qemu-raspi3b.sh).

1. **Unmodified daily image:** U-Boot → bootloader → kernel, then
   `KERNEL PANIC ... src/dtb/irqchip.rs:133:48: MissingRequiredProperty("interrupt-cells")`
   while parsing the GPIO node.
2. **With the fixed kernel:** boots to `redox login:`, login as `root` works,
   the `ion` shell runs. 408 MB RAM free after loading the 509 MiB live disk.
3. Commands like `uname`, `ls`, `cat`, `ps`, `free`, `df` crash immediately
   (see "relibc startup crash" below). `pcid` crashes at boot because there
   is no PCI bus (harmless, but noisy).

## Bug 1: kernel panic on the Pi's devicetree (fixed here)

**Cause:** Kernel commit `e595f73` (2026-10-17, "Update fdt crate") switched
to `fdt` rev `8fa4388`. In that version, a node without its own
`interrupt-parent` gets its **direct devicetree parent** as interrupt parent.
On the Pi, the GPIO controller's parent is the bus node `/soc`, which has no
`#interrupt-cells`; the real interrupt parent is inherited from the root node.
Linux (`of_irq_find_parent`) keeps walking up the tree until it reaches an
interrupt controller.

**Affected:** every Raspberry Pi 3 / Zero 2 W devicetree. Reproduced on the
host with the official `bcm2710-rpi-3-b.dtb`, `bcm2710-rpi-3-b-plus.dtb`
and `bcm2710-rpi-zero-2-w.dtb`. Redox's own hand-written Pi 3B+ DTS in
redox_firmware has no GPIO node and is not affected.

**Fix:** [`patches/fdt/0001-…`](../patches/fdt/) for the
[`fdt` crate](https://github.com/repnop/fdt) (third party, not part of Redox):
walk `interrupt-parent` links and devicetree parents until a node with
`#interrupt-cells` is found. Includes a regression test with a small
self-written DTS. All crate tests pass.

Use it in a kernel build with:

```sh
cargo rustc ... --config 'patch."https://github.com/repnop/fdt.git".fdt.path="<path to patched fdt>"'
```

## Bug 2: relibc startup crash (upstream, not Pi-specific)

Programs started from `ion` die with `brk #1` (ESR `0xF2000001`) in
`relibc_start_v1`, panic message `no filetable fd present`
(relibc `src/start.rs:183`). Same result on QEMU `virt` with `-cpu max` and
the unmodified kernel, so it is a general regression in the 2026-10-24 daily
build. Likely related to the `dup2` / filetable changes in relibc and base
this week. Next step: retry with a newer daily image or a self-built image.

## Implications for the Zero 2 W

- **512 MB RAM rules out live mode** with the server image (509 MiB). The
  Zero 2 W needs a smaller image in live mode, or the SD card driver.
- The prebuilt U-Boot embeds a Pi 3B DT. For the Zero 2 W we need a U-Boot
  that uses the firmware's DT (upstream `rpi_arm64_defconfig`, `OF_BOARD`),
  or a Zero 2 W build.
- In the official Zero 2 W DT the SD card is on `sdhost`
  (`brcm,bcm2835-sdhost`, `mmc@7e202000`) and the Arasan controller that
  Redox's driver uses is `disabled`. Either route the SD card to the Arasan
  controller (Pi firmware overlay `dtoverlay=mmc`, to be verified) or write
  an sdhost driver.
- The DWC2 USB controller is `disabled` in the base DT and is enabled by the
  firmware (`dtoverlay=dwc2` or `otg_mode=1`, to be verified).

## Tools in this repository

- [`tools/redoxfs-file`](../tools/redoxfs-file): read or replace a file in the
  RedoxFS partition of an image, without FUSE (works on macOS).
- [`scripts/qemu-raspi3b.sh`](../scripts/qemu-raspi3b.sh): download the latest
  daily server image, optionally inject a kernel, boot in QEMU `raspi3b`.
