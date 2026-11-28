# Boot-time probes (#47)

Measurements behind [docs/boot-time.md](../../docs/boot-time.md). Run them in your own
build clone on the build host, after `scripts/build-image.sh` and
`scripts/fw-direct-boot.sh` (which writes `build/fw-direct`). Copy this directory to
`<clone>/bt/`, since `mkmini.sh` uses paths relative to it. One QEMU at a time per clone.

- `tsboot.py <dir> <log> [regex] [timeout] [append]` boots `kernel8.img`, `initramfs` and
  `qemu.dtb` from `<dir>` in QEMU raspi3b, and writes every serial line with the seconds
  since QEMU started. It stops when `regex` appears (default `redox login:`).
  `INIT_LOG_LEVEL=DEBUG` as the append string makes init print each `Starting …` line.
- `hello-app.c` is the probe app. It prints `APP READY` and its monotonic time, then sleeps.
  Build it with the cookbook's cross gcc:
  `upstream/redox/prefix/aarch64-unknown-redox/gcc-install/bin/aarch64-unknown-redox-gcc -O2 -static -o hello-app hello-app.c`
- `mkmini.sh <dir> yes|no <unit>...` copies the built initfs tree and keeps the app plus
  the units you name. The app requires `00_runtime.target` and those units, and
  `90_initfs.target` only requires the app. It repacks the result with
  `redox-initfs-ar` into `<dir>`. With `yes`, the RedoxFS live disk is appended.

Example:

```bash
./mkmini.sh v-sd yes 40_rpi-firmwared.service 40_bcm2835-sdhostd.service
python3 tsboot.py v-sd v-sd-1.log "APP READY" 60 INIT_LOG_LEVEL=DEBUG
```
