# Experiment `pi400-mem`: all RAM on the Pi 4 and 400

WP 3.2 #299. `config/config-pi4.txt` sets `total_mem=1024`, so the Pi 400
runs Redox in less than 1 of its 4 GiB. The reason is DMA: on the BCM2711
the legacy peripherals (firmware mailbox, DMA engine, HDMI audio) reach RAM
through `/soc`'s `dma-ranges`, which cover only the first GiB. The PCIe
inbound window (VL805, xhcid) covers 3 GiB.

Kernel patch `0001`: a DMA zone, like Linux's ZONE_DMA.
- On a devicetree with `brcm,bcm2711`, and only when RAM goes past 1 GiB,
  the kernel takes 64 MiB in 4 MiB blocks below 1 GiB out of the page
  allocator at boot.
- Every physically contiguous allocation (`zeroed@…?phys_contiguous`, which
  `common::Dma` uses) comes from that pool, and freeing returns it there.
  Drivers don't change.
- The boot log says `DMA zone: … KiB below 0x40000000`. A full pool gives a
  warning and ENOMEM, never memory above the limit.

To use all RAM, the Pi's `config.txt` also needs the `total_mem=1024` line
removed. The image alone doesn't change the memory size.

## Test

- QEMU `raspi4b` (2 GiB): the boot log line, the boot reaches login, and
  `free` shows about 2 GiB.
- Pi 400 without `total_mem`: U-Boot `DRAM:` about 3.9 GiB, keyboard, GENET,
  rpi-fb and `pi-test.py`, compared with Test 531 (the same boot without
  this patch).
