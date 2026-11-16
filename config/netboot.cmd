# U-Boot script: boot Redox over the network.
#
# Loads a gzip-compressed Redox disk image over HTTP (falls back to TFTP),
# unpacks it in RAM,
# exposes it as a block device with blkmap and starts the live bootloader from
# TFTP, which copies the image into memory for the kernel. Compiled into
# netboot.scr by scripts/netboot.sh, after the boot menu (config/bootmenu.cmd).
#
# Variables (defaults set by scripts/netboot.sh, override in the environment):
#   redox_server   TFTP server (IP address)
#   redox_http     HTTP URL of the same directory, with a trailing slash;
#                  much faster than TFTP, which waits for every block;
#                  "none" loads everything over TFTP
#   redox_dir      directory on the TFTP server, with a trailing slash
#   redox_img_addr RAM address for the disk image
#   redox_gz_addr  RAM address for the compressed image, after the disk image
#   redox_netdiag  optional command run after each failed load, e.g. a dump of
#                  the network controller's counters (set for the Pi 4 and
#                  400 by scripts/netboot.sh)
#
# Each file is tried three times over HTTP, then three times over TFTP: on
# the Pi 400 a load over GENET fails at random and the next try succeeds
# (#394).

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
setenv redox_loaded no
for try in 1 2 3; do
    if test "${redox_loaded}" = no && test "${redox_http}" != none; then
        if wget ${redox_gz_addr} ${redox_http}${redox_dir}harddrive.img.gz; then
            setenv redox_loaded yes
            echo "redox-rpi: image loaded over HTTP"
        else
            echo "redox-rpi: HTTP load of harddrive.img.gz failed, try ${try} of 3"
            if test -n "${redox_netdiag}"; then run redox_netdiag; fi
            sleep 1
        fi
    fi
done
for try in 1 2 3; do
    if test "${redox_loaded}" = no; then
        setenv serverip ${redox_server}
        if tftpboot ${redox_gz_addr} ${redox_dir}harddrive.img.gz; then
            setenv redox_loaded yes
        else
            echo "redox-rpi: TFTP load of harddrive.img.gz failed, try ${try} of 3"
            if test -n "${redox_netdiag}"; then run redox_netdiag; fi
            sleep 1
        fi
    fi
done
if test "${redox_loaded}" = no; then exit; fi
unzip ${redox_gz_addr} ${redox_img_addr} || exit
setexpr redox_blocks ${filesize} / 0x200
blkmap create redox
blkmap map redox 0 ${redox_blocks} mem ${redox_img_addr}
# Reading the partition table probes the device, which makes U-Boot register
# it and its partitions as UEFI block devices for the Redox bootloader.
part list blkmap 0

setenv redox_loaded no
for try in 1 2 3; do
    if test "${redox_loaded}" = no && test "${redox_http}" != none; then
        if wget ${kernel_addr_r} ${redox_http}${redox_dir}bootloader-live.efi; then
            setenv redox_loaded yes
        else
            echo "redox-rpi: HTTP load of bootloader-live.efi failed, try ${try} of 3"
            if test -n "${redox_netdiag}"; then run redox_netdiag; fi
            sleep 1
        fi
    fi
done
for try in 1 2 3; do
    if test "${redox_loaded}" = no; then
        setenv serverip ${redox_server}
        if tftpboot ${kernel_addr_r} ${redox_dir}bootloader-live.efi; then
            setenv redox_loaded yes
        else
            echo "redox-rpi: TFTP load of bootloader-live.efi failed, try ${try} of 3"
            if test -n "${redox_netdiag}"; then run redox_netdiag; fi
            sleep 1
        fi
    fi
done
if test "${redox_loaded}" = no; then exit; fi
bootefi ${kernel_addr_r} ${fdtcontroladdr}
