# Experiment: late DHCP (test only)

Starts `dhcpd` 90 seconds late, so QEMU shows what a real Pi sometimes does:
the network comes up after `timesync` has started. Used to test the
`timesync` fix for test #27, where `timesync` hung in its DNS lookup.

```bash
scripts/build-image.sh --sdcard --experiment timesync --experiment late-dhcp
scripts/qemu-raspi3b.sh --usb-net --image <harddrive.img> --run "sleep 150"
```
