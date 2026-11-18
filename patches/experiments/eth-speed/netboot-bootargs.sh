#!/usr/bin/env bash
# Pass words on the kernel's command line on network boot (#126), for
# example RPI_ETH_SPEED=100. The test Pi loads its firmware from the SD
# card, so cmdline.txt on the TFTP server is never read. Instead this adds
# `setenv bootargs "${bootargs} <words>"` before `bootefi` in the staged
# netboot.cmd and compiles netboot.scr again; U-Boot writes bootargs into
# the devicetree's /chosen bootargs for the Redox bootloader. Restoring the
# stable image (cp -p /srv/tftp/stable/* /srv/tftp/redox/) undoes it.
#
# Usage: patches/experiments/eth-speed/netboot-bootargs.sh "<words>" [<tftp dir>]
#        (default dir /srv/tftp/redox; "" as words removes the line)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
words="${1?words}"
dir="${2:-/srv/tftp/redox}"
mkimage="$ROOT/build/u-boot/obj/tools/mkimage"
[ -x "$mkimage" ] || mkimage="$(command -v mkimage)" || { echo "no mkimage, run scripts/build-uboot.sh rpi" >&2; exit 1; }
cmd="$dir/netboot.cmd"
grep -q '^bootefi ' "$cmd" || { echo "$cmd has no bootefi line" >&2; exit 1; }

# The marker comment sits on its own line: the line after it is ours.
sed -i '/^# netboot-bootargs.sh$/,+1d' "$cmd"
if [ -n "$words" ]; then
    sed -i "/^bootefi /i # netboot-bootargs.sh\nsetenv bootargs \"\${bootargs} $words\"" "$cmd"
fi
"$mkimage" -q -A arm64 -T script -C none -n "redox-rpi netboot" -d "$cmd" "$dir/netboot.scr"
grep -n "netboot-bootargs.sh\|^bootefi" "$cmd"
