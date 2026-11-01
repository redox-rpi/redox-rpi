# Experiment: rpi-firmwared on fdt 0.2

Upstream base moved to fdt `0.2.0-alpha3` in `919f8268` (2026-10-30). This
patch moves `rpi-firmwared` from its own
`fdt = { git = "https://github.com/repnop/fdt.git", rev = "059bb23" }` to
the workspace `fdt` and the 0.2 API, with the pattern of experiment
`sdhost-fdt02` and [docs/upstream/rebase.md](../../../docs/upstream/rebase.md)
("Update 2026-11-01"). Only `mbox_phys()` and the `Fdt::new` call change:
`all_compatible`, `reg()`/`ranges()` iterators with `Result` items,
`address`/`len`, `Fdt::new_unaligned`, and a type alias for the `Fdt`
passed to `mbox_phys`.

**Only for a base at `919f8268` or later.** With the pinned base
(`eddcaa21`, workspace `fdt = "0.1.5"`) the patch applies but does not
compile. Don't select it until `config/upstream.lock` moves base; then it
belongs into `patches/base/` (or folded into `0018`). Without it,
`rpi-firmwared` still builds on the new base, with the old fdt as a second
crate.

## Drift check, 2026-11-01

Upstream base `8424d392` with main's default base patches in order (`0001`
and `0018` from PR #124): `0018`, `0021`, `0040` and `0046`, the
`rpi-firmwared` patches, apply as they are (plain `git am`, no 3-way). Of
the others, `0015` (sdhcid) and `0024` (getty) conflict; they are not
firmware patches.

## Results

2026-11-01, homenodellm:

- `redoxer build --release -p rpi-firmwared` in upstream base `8424d392`
  with the default base patches (`0015` and `0024` left out, they
  conflict) and this patch: no errors, no warnings in the driver;
  `Cargo.lock` lists `fdt 0.2.0-alpha3` for `rpi-firmwared`.
- `mbox_phys()` as patched, built on the host with `fdt = "=0.2.0-alpha3"`
  and run on the firmware's devicetrees: `bcm2710-rpi-3-b-plus.dtb` and
  `bcm2710-rpi-zero-2-w.dtb` both give `0x3f00b880`, the address the
  driver uses today (from `/soc` `ranges`; without them it would be the
  bus address `0x7e00b880`).
- Started by hand in QEMU, the second instance printed nothing: the daemon
  library refuses a manual start (`INIT_NOTIFY`) and, with it set, the
  forked child's output did not reach the terminal. So no QEMU run of this
  binary; the image test belongs to the lock move.

Not done: a whole image on the new base and a Pi test.
