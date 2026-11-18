# U-Boot script: boot menu (#243). scripts/netboot.sh puts it into
# netboot.scr and into boot-local.scr (boot.scr on cards with Redox on them,
# scripts/make-sdcard.sh --local).
#
# Entries only choose; the script boots afterwards. The default is chosen
# before the menu shows, so the timeout, "Exit" and a U-Boot without the
# bootmenu command all boot the default.
#
# Variables (override in the environment):
#   redox_dir           network directory of this script, with a trailing slash
#   redox_menu_delay    seconds before the default boots (default 3,
#                       -1 waits for a key, 0 skips the menu)
#   redox_menu_default  "card" or "net": the default where both exist
#                       (default card)

if test -z "${redox_menu_done}"; then
    setenv redox_menu_done 1
    if test -z "${redox_menu_delay}"; then setenv redox_menu_delay 3; fi

    # The card U-Boot ran this script from, if it has the Redox bootloader.
    setenv redox_card
    if test -n "${devtype}" && test -e ${devtype} ${devnum}:${distro_bootpart} EFI/BOOT/BOOTAA64.EFI; then
        setenv redox_card ${devnum}:${distro_bootpart}
    fi

    if test "${redox_dir}" = stable/; then setenv redox_other redox/; else setenv redox_other stable/; fi
    setenv redox_choice net
    setenv bootmenu_default 0
    if test -n "${redox_card}"; then
        setenv bootmenu_0 "This SD card=setenv redox_choice card"
        setenv bootmenu_1 "Network: ${redox_dir}=setenv redox_choice net"
        setenv bootmenu_2 "Network: ${redox_other}=setenv redox_choice net; setenv redox_dir ${redox_other}"
        setenv bootmenu_3
        if test "${redox_menu_default}" = net; then
            setenv bootmenu_default 1
        else
            setenv redox_choice card
        fi
    else
        setenv bootmenu_0 "Network: ${redox_dir}=setenv redox_choice net"
        setenv bootmenu_1 "Network: ${redox_other}=setenv redox_choice net; setenv redox_dir ${redox_other}"
        setenv bootmenu_2
    fi
    if test "${redox_menu_delay}" != 0; then
        bootmenu ${redox_menu_delay}
    fi
    echo "redox-rpi: boot ${redox_choice} ${redox_dir}"

    if test "${redox_choice}" = card; then
        load ${devtype} ${redox_card} ${kernel_addr_r} EFI/BOOT/BOOTAA64.EFI && bootefi ${kernel_addr_r} ${fdtcontroladdr}
        exit
    fi
fi
