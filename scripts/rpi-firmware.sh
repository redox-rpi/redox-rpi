# Raspberry Pi firmware files for the boot partition, shared by
# scripts/make-sdcard.sh and scripts/netboot-firmware.sh. Sourced, not run.
#
# fetch_firmware <root> downloads the files into
# <root>/build/firmware/$FIRMWARE_TAG once and sets $firmware_cache to it.

# Raspberry Pi firmware release, see github.com/raspberrypi/firmware/tags
FIRMWARE_TAG="1.20260915"
FIRMWARE_URL="https://raw.githubusercontent.com/raspberrypi/firmware/$FIRMWARE_TAG/boot"
FIRMWARE_FILES=(
    bootcode.bin start.elf fixup.dat start_cd.elf fixup_cd.dat LICENCE.broadcom
    bcm2710-rpi-3-b.dtb bcm2710-rpi-3-b-plus.dtb bcm2710-rpi-zero-2-w.dtb
    overlays/disable-bt.dtbo overlays/mmc.dtbo
)

fetch_firmware() {
    firmware_cache="$1/build/firmware/$FIRMWARE_TAG"
    local file
    for file in "${FIRMWARE_FILES[@]}"; do
        if [ ! -f "$firmware_cache/$file" ]; then
            mkdir -p "$(dirname "$firmware_cache/$file")"
            curl -fsSL -o "$firmware_cache/$file" "$FIRMWARE_URL/$file"
        fi
    done
}
