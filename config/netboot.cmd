# U-Boot script: boot Redox over the network.
#
# Loads a gzip-compressed Redox disk image over HTTP (falls back to TFTP),
# unpacks it in RAM,
# exposes it as a block device with blkmap and starts the live bootloader from
# TFTP, which copies the image into memory for the kernel. Compiled into
# netboot.scr by scripts/netboot.sh.
#
# Variables (defaults set by scripts/netboot.sh, override in the environment):
#   redox_server   TFTP server (IP address)
#   redox_http     HTTP URL of the same directory, with a trailing slash;
#                  much faster than TFTP, which waits for every block;
#                  "none" loads everything over TFTP
#   redox_dir      directory on the TFTP server, with a trailing slash
#   redox_img_addr RAM address for the disk image
#   redox_gz_addr  RAM address for the compressed image, after the disk image

echo "redox-rpi: network boot from ${redox_server}"

# Ethernet on the Pi is a USB device. Harmless where there is no USB.
# boot.scr (scripts/netboot.sh) has started USB and asked DHCP already when
# it loads this script; asking again costs seconds on the Pi (#47).
if test -z "${ipaddr}"; then
    usb start
    setenv autoload no
    dhcp || exit
fi
# DHCP may set serverip from the router; ours wins.
setenv serverip ${redox_server}

# The image is mostly empty space; compressed it is about a sixth.
if test "${redox_http}" != none && wget ${redox_gz_addr} ${redox_http}${redox_dir}harddrive.img.gz; then
    echo "redox-rpi: image loaded over HTTP"
else
    setenv serverip ${redox_server}
    tftpboot ${redox_gz_addr} ${redox_dir}harddrive.img.gz || exit
fi
unzip ${redox_gz_addr} ${redox_img_addr} || exit
setexpr redox_blocks ${filesize} / 0x200
blkmap create redox
blkmap map redox 0 ${redox_blocks} mem ${redox_img_addr}
# Reading the partition table probes the device, which makes U-Boot register
# it and its partitions as UEFI block devices for the Redox bootloader.
part list blkmap 0

if test "${redox_http}" != none && wget ${kernel_addr_r} ${redox_http}${redox_dir}bootloader-live.efi; then
    true
else
    setenv serverip ${redox_server}
    tftpboot ${kernel_addr_r} ${redox_dir}bootloader-live.efi || exit
fi
bootefi ${kernel_addr_r} ${fdtcontroladdr}
