# Experiment: rpi-fb-dma-on, a test image with DMA blits on (#33)

Only for Pi tests of `rpi-fbd`'s DMA blits. Use it together with `rpi-fb`:

```bash
scripts/build-image.sh --experiment timesync --experiment rpi-fb --experiment rpi-fb-dma-on
```

`rpi-fbd` blits with the CPU unless `RPI_FB_DMA=on` is set (patch `0070`).
Netboot can't pass it on the Pi used for tests: the firmware reads its
`cmdline.txt` from the SD card. [`0073`](base/0073-rpi-fbd-DMA-blits-on-in-the-service-file-for-a-test-.patch)
sets it in `rpi-fbd`'s service file. Not meant for `main`.
