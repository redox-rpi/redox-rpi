# Experiment `pi400-mem`: all RAM on the Pi 4 and 400

WP 3.2 #299. `config/config-pi4.txt` used to set `total_mem=1024`, so the
Pi 400 ran Redox in less than 1 of its 4 GiB. The reason was DMA: on the BCM2711
the legacy peripherals (firmware mailbox, DMA engine, HDMI audio) reach RAM
through `/soc`'s `dma-ranges`, which cover only the first GiB. The PCIe
inbound window (VL805, xhcid) covers 3 GiB.

Kernel patch `0001`: a DMA zone, like Linux's ZONE_DMA.
- On a devicetree with `brcm,bcm2711`, and only when RAM goes past 1 GiB,
  the kernel takes 256 MiB in blocks of MAX_ORDER size (4 MiB, or 16 MiB
  with rpi-fb's kernel patch 0049) below 1 GiB out of the page
  allocator at boot.
- 256 MiB, as Linux's CMA on the Pi 4 with vc4-kms: vc4d's buffers are
  8.3 MB each at 1080p, and one buffer can't span two blocks (so it needs
  the 16 MiB blocks of 0049). Tests 531–536 ran with 64 MiB.
- Every physically contiguous allocation (`zeroed@…?phys_contiguous`, which
  `common::Dma` uses) comes from that pool, and freeing returns it there.
  Drivers don't change.
- The boot log says `DMA zone: … KiB below 0x40000000`. A full pool gives a
  warning and ENOMEM, never memory above the limit.

The same PR removes `total_mem=1024` from `config/config-pi4.txt`. The
image alone doesn't change the memory size: on a Pi whose `config.txt`
still has the line, Redox sees 947 MiB and the kernel makes no pool.

## Test

- QEMU `raspi4b`: it gives the guest only 960 MiB, so the kernel logs
  `all RAM … is below the DMA limit, no pool`. A temporary variant with
  the limit at 512 MiB and a 16 MiB pool booted to login.
- Pi 400, all without `total_mem`, A without and B with this patch:
  - Test 531 (A) and 532 (B): U-Boot `DRAM: 3.9 GiB`, login, xhcid and the
    keyboard. B logs `DMA zone: 65536 KiB below 0x40000000` and uses
    about 60 MB more.
  - Test 535: GENET in both (link 1000, DHCP, pi-test tcp 10 MiB OK).
  - Test 536 (`hdmi_force_hotplug=1`): rpi-fbd 640x480 and fbshot in both.
  - Not tested: DMA-engine blits (`RPI_FB_DMA`, off by default) and HDMI
    audio with 4 GiB. Without the zone the mailbox works only because its
    buffer happens to be allocated early, from low memory.
