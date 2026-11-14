#!/usr/bin/env bash
# Prepare the TFTP directory so a Pi 3B+ without SD card boots from the
# network: its boot ROM fetches the Pi firmware over TFTP, the firmware
# loads U-Boot, and U-Boot continues as with the netboot SD card
# (redox/netboot.scr from scripts/netboot.sh).
#
# Only adds files; redox/ and everything else in the TFTP directory stays as
# it is. A Pi with an SD card in it keeps booting from the card.
#
#   <tftp-dir>/bootcode.bin           fetched by the boot ROM
#   <tftp-dir>/<serial>/start.elf ... fetched by bootcode.bin, see --serial
#
# The boot ROM needs a DHCP answer with "Raspberry Pi Boot" in it, which a
# home router does not send: run dnsmasq as a proxy next to the router with
# config/dnsmasq-pxe-proxy.conf (docs/netboot.md, "Without SD card").
#
# Run scripts/build-uboot.sh rpi-net <server-ip> first.
#
# Pi 4 family (--board pi4, Pi 400 test version, #299): the bootloader in
# the board's EEPROM does the network boot itself, without bootcode.bin. It
# fetches start4.elf, config.txt, the devicetree and U-Boot from
# <tftp-dir>/<prefix>/, where the EEPROM setting TFTP_PREFIX=1 with
# TFTP_PREFIX_STR=<prefix>/ names the directory (default prefix pi400-fw).
# The U-Boot there comes from scripts/build-uboot.sh rpi-net <server-ip>
# pi400 and loads pi400/netboot.scr (scripts/netboot.sh --board pi4 --dir
# pi400).
#
# Usage:
#   scripts/netboot-firmware.sh [--tftp-dir <dir>] [--serial <serial>]
#   scripts/netboot-firmware.sh --board pi4 [--tftp-dir <dir>] [--prefix <name>]
#
#   --tftp-dir <dir>  TFTP root (default: build/tftp)
#   --serial <serial> the Pi's serial number, 8 hex digits: bootcode.bin
#                     looks in <serial>/ first, then in the TFTP root. The
#                     TFTP server's log shows which name it asks for.
#                     Default: the TFTP root.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
. "$ROOT/scripts/rpi-firmware.sh"

tftp="$ROOT/build/tftp"
serial=""
board=pi3
prefix=pi400-fw
while [ $# -gt 0 ]; do
    case "$1" in
        --tftp-dir) tftp="$2"; shift 2 ;;
        --serial) serial="$2"; shift 2 ;;
        --board) board="$2"; shift 2 ;;
        --prefix) prefix="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done
case "$serial" in
    ""|[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]) ;;
    *) echo "--serial: 8 lowercase hex digits, for example 1a2b3c4d" >&2; exit 1 ;;
esac

if [ "$board" = pi4 ]; then
    uboot="$ROOT/build/u-boot-net-pi400/u-boot.bin"
    [ -f "$uboot" ] || { echo "$uboot missing, run scripts/build-uboot.sh rpi-net <server-ip> pi400" >&2; exit 1; }
    [ -f "$tftp/pi400/netboot.scr" ] || echo "warning: $tftp/pi400/netboot.scr missing, run scripts/netboot.sh --board pi4 --dir pi400" >&2
    fetch_firmware "$ROOT"
    dir="$tftp/$prefix"
    mkdir -p "$dir/overlays"
    for file in "${PI4_FIRMWARE_FILES[@]}"; do
        cp "$firmware_cache/$file" "$dir/$file"
    done
    cp "$ROOT/config/config-pi4.txt" "$dir/config.txt"
    cp "$uboot" "$dir/u-boot.bin"
    echo "Pi 4 family network boot ready: $dir/ (EEPROM: TFTP_PREFIX=1, TFTP_PREFIX_STR=$prefix/)"
    ls -l "$dir"
    exit 0
fi
[ "$board" = pi3 ] || { echo "unknown board: $board" >&2; exit 1; }

uboot="$ROOT/build/u-boot-net/u-boot.bin"
[ -f "$uboot" ] || { echo "$uboot missing, run scripts/build-uboot.sh rpi-net <server-ip>" >&2; exit 1; }
[ -f "$tftp/redox/netboot.scr" ] || echo "warning: $tftp/redox/netboot.scr missing, run scripts/netboot.sh" >&2

fetch_firmware "$ROOT"
dir="$tftp${serial:+/$serial}"
mkdir -p "$dir/overlays"
cp "$firmware_cache/bootcode.bin" "$tftp/bootcode.bin"
for file in "${FIRMWARE_FILES[@]}"; do
    [ "$file" = bootcode.bin ] || cp "$firmware_cache/$file" "$dir/$file"
done
cp "$ROOT/config/config.txt" "$dir/config.txt"
cp "$uboot" "$dir/u-boot.bin"

echo "Firmware network boot ready: $tftp/bootcode.bin, $dir/"
ls -l "$dir"
