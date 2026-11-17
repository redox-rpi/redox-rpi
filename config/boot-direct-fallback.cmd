# U-Boot fallback on the direct card (scripts/make-sdcard.sh --direct
# --uboot-fallback, #324). config.txt starts U-Boot instead of kernel8.img
# only when GPIO3 (header pin 5) is held to ground at power-on, or when
# config.txt was edited to kernel=u-boot.bin. U-Boot then starts the same
# kernel8.img (rpi-boot-stub with the kernel) and initramfs from this card
# with booti, so both ways run the same system.
if test -z "${devtype}"; then setenv devtype mmc; fi
if test -z "${devnum}"; then setenv devnum 0; fi
if test -z "${distro_bootpart}"; then setenv distro_bootpart 1; fi
if test -z "${fdt_addr}"; then setenv fdt_addr ${fdtcontroladdr}; fi
echo "redox-rpi: U-Boot fallback, starting kernel8.img and initramfs from ${devtype} ${devnum}:${distro_bootpart}"
# Leave the initramfs (about 250 MiB) where it is loaded instead of copying it.
setenv initrd_high 0xffffffffffffffff
fatload ${devtype} ${devnum}:${distro_bootpart} ${kernel_addr_r} kernel8.img
fatload ${devtype} ${devnum}:${distro_bootpart} ${ramdisk_addr_r} initramfs
booti ${kernel_addr_r} ${ramdisk_addr_r}:${filesize} ${fdt_addr}
echo "redox-rpi: booti failed"
