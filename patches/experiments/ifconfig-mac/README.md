# Experiment ifconfig-mac: now a default patch

Issues: [#171](https://github.com/redox-rpi/redox-rpi/issues/171),
[#444](https://github.com/redox-rpi/redox-rpi/issues/444).

The patch that makes `ifconfig` show the MAC address from
`/scheme/netcfg/ifaces/<name>/mac` (and the netmask as `255.255.255.0`) is
`patches/netutils/0002` now, in every image. Before, images without this
experiment printed upstream's placeholder `00:00:00:00:00:00`.

This directory stays, empty of patches, so that build commands with
`--experiment ifconfig-mac` (the v0.5.0 release commands) still work; the
option no longer changes anything. Drop it from new commands.

Tested as the experiment: QEMU `raspi3b`, Pi test 194
(`MAC Address: b8:27:eb:08:e0:aa`), v0.5.0-rc1 and v0.5.0 (Pi row 414).
