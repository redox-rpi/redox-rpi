# Experiment: bcm2835-sdhostd on fdt 0.2

Upstream base moved to fdt `0.2.0-alpha3` in `919f8268` (2026-10-30). This
patch moves `bcm2835-sdhostd` from its own
`fdt = { git = "https://github.com/repnop/fdt.git", rev = "059bb23" }` to
the workspace `fdt` and the 0.2 API, the same way upstream changed
`bcm2835-sdhcid` in that commit. It shows the change for the other drivers
that read the devicetree (dwc2d, rpi-firmwared, GPIO, I2C, SPI). The
before/after pattern is in
[docs/upstream/rebase.md](../../../docs/upstream/rebase.md), section
"Update 2026-11-01".

**Only for a base at `919f8268` or later.** With the pinned base
(`eddcaa21`, workspace `fdt = "0.1.5"`) the patch applies but doesn't
compile. Don't select it until `config/upstream.lock` moves base; then it
belongs into `patches/base/` (or folded into `0025`). Without the port, the
driver still builds on the new base, with the old fdt as a second crate.

## Results

2026-11-01, QEMU raspi3b, homenodellm:

- `cargo check` and `cargo build --release -p bcm2835-sdhostd --target
  aarch64-unknown-redox` in upstream base `8424d392` with our default base
  patches `0001`–`0028` (`0024` getty left out, it conflicts) and this
  patch: no errors, no warnings in the driver.
- That binary copied into the image of `v0.3.0-dev-880-g61d40ca+x12cb`
  (netboot, pinned base) as `/usr/bin/sdhostd02` and started by hand as
  `sdhostd02 disk.sdhost` next to the boot driver:

  ```text
  bcm2835-sdhostd: controller at 0x3f202000
  bcm2835-sdhostd: GPFSEL2 0x00000000 GPFSEL4 0x24000800 GPFSEL5 0x00000924
  bcm2835-sdhostd: core clock 350 MHz, up to 350 MHz
  bcm2835-sdhostd: SDSC card, 256 MiB, 4-bit bus, high speed, up to 50 MHz
  bcm2835-sdhostd: serving /scheme/disk.sdhost
  ```

  `0x3f202000` comes from the devicetree's `/soc` `ranges` (bus address
  `0x7e202000`), so the new `ranges` code works. The `reg` fallback has the
  same value as the node, so that part is checked by the build only.
  `ls /scheme/disk.sdhost` shows `0 0p0 0p1 0p2`; the first MiB read through
  it matches the card image (sha256 `77cb775d…`). The boot driver on
  `disk.mmc` then reinitialized the card once, as expected with two drivers
  on one controller.

Not done: a whole image on the new base (other patches still conflict, the
lock move is planned separately) and no Pi test.
