# rpi-boot-stub

A `kernel8.img` that the Raspberry Pi firmware starts directly. It starts the
unchanged Redox kernel without U-Boot and without the Redox bootloader
([#15](https://github.com/redox-rpi/redox-rpi/issues/15)).

```
today:   Pi firmware → U-Boot (UEFI) → Redox bootloader → kernel
stub:    Pi firmware → rpi-boot-stub → kernel
```

## What it does

The firmware loads `kernel8.img` at `0x80000` and the initramfs, and starts
the stub at EL2 with the MMU off and `x0` pointing to the devicetree. The
stub does what the Redox bootloader does on aarch64 before the jump:

1. EL2 (or EL3) to EL1, the same register values as the bootloader.
2. Memory areas for the kernel: `/memory` from the devicetree as free
   memory, minus everything below the end of the stub (firmware spin
   tables, stub, kernel, page tables, stack), the devicetree, the
   initramfs and `/memreserve/` entries.
3. The initramfs from `/chosen` (`linux,initrd-start`, `-end`): the Redox
   initfs, then, at the next 4 KiB boundary, an optional RedoxFS image. With
   one, it becomes the live disk (`DISK_LIVE_ADDR`, `DISK_LIVE_SIZE`,
   `REDOXFS_UUID` in the environment), as with `bootloader-live.efi`.
4. Page tables in the bootloader's layout: the first 8 GiB identity-mapped
   and at `PHYS_OFFSET`, the kernel at `KERNEL_OFFSET`.
5. MMU on and the jump to the kernel's entry point with `KernelArgs`.

The kernel ELF is built into the stub (`REDOX_KERNEL` at build time); the
kernel's linker script makes the file a flat image, so it runs in place.
It prints its steps on the PL011 UART (`rpi-boot-stub: …`).

## Build and test

```sh
rustup target add aarch64-unknown-none
scripts/fw-direct-boot.sh --image <harddrive.img> --test
```

`scripts/fw-direct-boot.sh` takes the kernel, the initfs and the RedoxFS
partition from an image of `scripts/build-image.sh`, writes
`build/fw-direct/kernel8.img` and `build/fw-direct/initramfs` and, with
`--qemu` or `--test`, boots them in QEMU `raspi3b` with `-kernel`, `-initrd`
and `-dtb`.

## State

- QEMU `raspi3b`: boots to the login prompt (2026-10-28). Four CPUs come
  up through the spin table, the root filesystem comes from the live disk.
- Real Pi: not tried yet. Needs `config.txt` with `kernel=kernel8.img`,
  `initramfs initramfs followkernel` and the same overlays as today.
- Not done yet: the Ethernet MAC address and the kernel command line from
  the firmware (U-Boot provides the MAC today), a `make-sdcard.sh` mode.
